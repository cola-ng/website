"""Account and subscription concurrency regressions: isolated loopback QA DB only."""
import concurrent.futures, json, os, shutil, subprocess, uuid
from urllib.request import Request, urlopen
from urllib.error import HTTPError
from urllib.parse import urlsplit
base, database = os.environ["QA_API"], os.environ["QA_DATABASE_URL"]
assert urlsplit(database).hostname in ("localhost", "127.0.0.1") and urlsplit(database).path == "/colang_qa"
assert urlsplit(base).hostname in ("localhost", "127.0.0.1")
def sql(query):
    return subprocess.check_output([shutil.which("psql"), database, "-XAt", "-v", "ON_ERROR_STOP=1", "-c", query], text=True).strip().splitlines()
def call(method, path, body=None, token=None):
    headers={"Content-Type":"application/json"}
    if token: headers["Authorization"]="Bearer "+token
    try: response=urlopen(Request(base+path, data=json.dumps(body).encode() if body is not None else None, headers=headers, method=method), timeout=30)
    except HTTPError as error: response=error
    raw=response.read()
    return response.status, json.loads(raw) if raw else None
email="integrity-"+uuid.uuid4().hex+"@example.test"
status,account=call("POST","/api/register",{"name":"Integrity QA","email":email,"password":"OriginalPassword123!"})
assert status==200,(status,account)
token=account["access_token"]; uid=account["user"]["id"]
status,_=call("POST","/api/me/change-password",{"current_password":"wrong","new_password":"NewPassword123!"},token)
assert status==401,status
# Force a write failure. The original password must survive the entire replacement.
sql(f"CREATE FUNCTION qa_password_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.user_id={uid} THEN RAISE EXCEPTION 'QA injected failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER qa_password_failure BEFORE INSERT OR UPDATE ON base_passwords FOR EACH ROW EXECUTE FUNCTION qa_password_failure();")
try:
    status,_=call("POST","/api/me/change-password",{"current_password":"OriginalPassword123!","new_password":"NewPassword123!"},token)
    assert status==503,status
finally:
    sql("DROP TRIGGER qa_password_failure ON base_passwords; DROP FUNCTION qa_password_failure();")
status,_=call("POST","/api/login",{"email":email,"password":"OriginalPassword123!"})
assert status==200,status
status,changed=call("POST","/api/me/change-password",{"current_password":"OriginalPassword123!","new_password":"NewPassword123!"},token)
assert status==200,(status,changed)
assert call("GET","/api/me",token=token)[0]==401
new_token=changed["access_token"]
assert call("GET","/api/me",token=new_token)[0]==200
with concurrent.futures.ThreadPoolExecutor(max_workers=20) as pool:
    statuses=list(pool.map(lambda _:call("GET","/api/wallet",token=new_token)[0],range(20)))
assert statuses==[200]*20,statuses
assert sql(f"SELECT count(*) FROM wallet_accounts WHERE user_id={uid}")[0]=="1"
code="qa-"+uuid.uuid4().hex[:24]
sql(f"INSERT INTO subscription_plans(code,name_en,name_zh,tier,price_credits,trial_days) VALUES('{code}','QA','QA','pro',100,7)")
with concurrent.futures.ThreadPoolExecutor(max_workers=20) as pool:
    statuses=list(pool.map(lambda _:call("POST","/api/subscription/subscribe",{"plan_code":code},new_token)[0],range(20)))
assert statuses.count(200)==1 and statuses.count(409)==19,statuses
assert sql(f"SELECT count(*) FROM user_subscriptions WHERE user_id={uid} AND status IN ('trial','active')")[0]=="1"
assert call("POST","/api/subscription/cancel",{},new_token)[0]==200
assert call("POST","/api/subscription/subscribe",{"plan_code":code},new_token)[0]==409
# A past expiry must not report current entitlement or block the next free plan.
sql(f"UPDATE user_subscriptions SET status='trial',expires_at=NOW()-INTERVAL '1 day' WHERE user_id={uid}")
status,current=call("GET","/api/subscription/current",token=new_token)
assert status==200 and current["has_subscription"] is False,(status,current)
free=sql("SELECT code FROM subscription_plans WHERE tier='free' AND is_active LIMIT 1")[0]
assert call("POST","/api/subscription/subscribe",{"plan_code":free},new_token)[0]==200
print("Review integrity: password rollback/token revocation, 20-way wallet and subscription concurrency, one-time trial and expiry passed")
