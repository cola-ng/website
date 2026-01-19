-- Rollback: Remove seed data from dict_dictionaries table

DELETE FROM dict_dictionaries WHERE short_en IN (
    'CET4',
    'CET6',
    'TEM4',
    'TEM8',
    'NEEP',
    'OED',
    'Webster',
    'Collins',
    'CALD',
    'Macmillan'
);
