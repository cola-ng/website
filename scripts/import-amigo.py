"""Import legacy local robot data into the website DB, preserving website accounts.

Run only after the website migrations and a verified database backup. Credentials
are read locally and passed to psql through its environment, never printed.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
from urllib.parse import unquote, urlparse

ROOT = Path(__file__).resolve().parents[1]
PSQL = 'C:/Program Files/PostgreSQL/18/bin/psql.exe' if os.name == 'nt' else 'psql'


def env_file(path):
    return {k.strip(): v.strip().strip('"\'') for k, v in
            (line.split('=', 1) for line in Path(path).read_text(encoding='utf-8-sig').splitlines()
             if '=' in line and not line.lstrip().startswith('#'))}


def query(url, sql):
    u = urlparse(url)
    if u.hostname not in ('localhost', '127.0.0.1', '::1'):
        raise ValueError('This migration accepts local databases only')
    env = os.environ.copy()
    env['PGPASSWORD'] = unquote(u.password or '')
    proc = subprocess.run([PSQL, '-h', u.hostname, '-p', str(u.port or 5432),
                           '-U', unquote(u.username or ''), '-d', u.path.lstrip('/'),
                           '-X', '-qAt', '-v', 'ON_ERROR_STOP=1'],
                          input=sql, env=env, capture_output=True, text=True, encoding='utf-8')
    if proc.returncode:
        # Never include the failing SQL / serialized rows in an exception.
        raise RuntimeError('Database operation failed; no credentials or rows were logged')
    return proc.stdout.strip()


def literal(value):
    return "'" + value.replace("'", "''") + "'"


def columns(url, table):
    return json.loads(query(url, "SELECT json_agg(column_name ORDER BY ordinal_position) "
                            "FROM information_schema.columns WHERE table_schema='public' AND table_name=" + literal(table)))


def run(source, destination):
    if source == destination:
        raise ValueError('Source and destination must differ')
    tables = ['amigo_devices', 'amigo_profiles', 'amigo_events', 'amigo_device_profiles',
              'learn_observations', 'learn_person_memories']
    rows = {table: json.loads(query(source, f"SELECT COALESCE(json_agg(t),'[]'::json) FROM {table} t")) for table in tables}
    owners = {r['user_id'] for r in rows['amigo_devices']}
    if not owners:
        raise ValueError('No legacy devices found')
    users = json.loads(query(source, "SELECT json_agg(t) FROM base_users t WHERE id IN (" + ','.join(map(str, sorted(owners))) + ')'))
    script = ['BEGIN;', 'CREATE TEMP TABLE amigo_import_users(old_id BIGINT PRIMARY KEY,new_id BIGINT NOT NULL,was_created BOOLEAN NOT NULL);']
    common = [c for c in columns(source, 'base_users') if c in columns(destination, 'base_users') and c not in ('id', 'handle')]
    for user in users:
        if not user.get('email'):
            raise ValueError('Legacy device owner needs an email for safe account mapping')
        email = literal(user['email'].lower())
        data = literal(json.dumps(user, ensure_ascii=False))
        cols = ','.join('"' + c + '"' for c in common)
        projections = ','.join('r."' + c + '"' for c in common)
        script.append(f"WITH existing AS (SELECT id FROM base_users WHERE lower(email)={email}), created AS (INSERT INTO base_users({cols}) SELECT {projections} FROM json_populate_record(NULL::base_users,{data}::json) r WHERE NOT EXISTS(SELECT 1 FROM existing) RETURNING id) INSERT INTO amigo_import_users SELECT {int(user['id'])},id,false FROM existing UNION ALL SELECT {int(user['id'])},id,true FROM created;")
    passwords = json.loads(query(source, "SELECT COALESCE(json_agg(t),'[]'::json) FROM base_passwords t WHERE user_id IN (" + ','.join(map(str, sorted(owners))) + ')'))
    for password in passwords:
        data = literal(json.dumps(password, ensure_ascii=False))
        script.append(f"INSERT INTO base_passwords(user_id,hash,created_at) SELECT m.new_id,r.hash,r.created_at FROM json_populate_record(NULL::base_passwords,{data}::json) r JOIN amigo_import_users m ON m.old_id=r.user_id WHERE m.was_created AND NOT EXISTS(SELECT 1 FROM base_passwords p WHERE p.user_id=m.new_id);")
    for device in rows['amigo_devices']:
        # A UUID collision must never transfer someone else's device.
        script.append("DO $$ BEGIN IF EXISTS (SELECT 1 FROM amigo_devices d JOIN amigo_import_users m ON m.old_id=" + str(device['user_id']) + " WHERE d.id=" + literal(device['id']) + " AND d.user_id<>m.new_id) THEN RAISE EXCEPTION 'Device owner conflict'; END IF; END $$;")
    for table in tables:
        cols = [c for c in columns(source, table) if c in columns(destination, table)]
        if table.startswith('learn_'):
            cols.remove('id')
        colsql = ','.join('"' + c + '"' for c in cols)
        for row in rows[table]:
            if 'user_id' in row and row['user_id'] not in owners:
                continue
            if table == 'amigo_device_profiles' and row['device_id'] not in {d['id'] for d in rows['amigo_devices']}:
                continue
            projections = ','.join('m.new_id' if c == 'user_id' else 'NULL' if c in ('lease_id', 'lease_until') else 'r."' + c + '"' for c in cols)
            source_sql = f"json_populate_record(NULL::{table},{literal(json.dumps(row, ensure_ascii=False))}::json) r"
            if 'user_id' in row:
                source_sql += f" JOIN amigo_import_users m ON m.old_id={int(row['user_id'])}"
            where = ''
            if table == 'learn_observations':
                where = ' WHERE NOT EXISTS (SELECT 1 FROM learn_observations e WHERE e.user_id=m.new_id AND e.source=r.source AND e.redacted_text=r.redacted_text AND e.observed_at=r.observed_at)'
            script.append(f"INSERT INTO {table}({colsql}) SELECT {projections} FROM {source_sql}{where} ON CONFLICT DO NOTHING;")
    script.append("SELECT json_build_object('mapped_accounts',(SELECT count(*) FROM amigo_import_users),'active_devices',(SELECT count(*) FROM amigo_devices WHERE revoked_at IS NULL),'events',(SELECT count(*) FROM amigo_events)); COMMIT;")
    print(query(destination, '\n'.join(script)))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', default='postgresql://amigo_test@127.0.0.1:18190/amigo_local')
    args = parser.parse_args()
    run(args.source, env_file(ROOT / '.env')['DATABASE_URL'])
