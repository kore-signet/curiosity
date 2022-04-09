-- example query
-- SELECT 
--     title, 
--     ts_headline('english', body, query, 'MaxFragments=2,MaxWords=35') AS excerpt,
--     ts_rank_cd(body_index, query) AS rank,
--     FROM episodes, to_tsquery('catgirl') query
--     WHERE query @@ body_index
-- ORDER BY rank DESC;

CREATE TEXT SEARCH DICTIONARY english_hunspell ( TEMPLATE = ispell, DictFile = en_us, AffFile = en_us, StopWords = english);
CREATE TEXT SEARCH CONFIGURATION public.fatt ( COPY = pg_catalog.english );
ALTER TEXT SEARCH CONFIGURATION fatt ALTER MAPPING FOR asciiword, asciihword, hword_asciipart, word, hword, hword_part WITH  english_hunspell, english_stem;

CREATE TABLE episodes (
    id text PRIMARY KEY,
    season text,
    title text,
    body text,
    body_index tsvector GENERATED ALWAYS AS (to_tsvector('fatt', body)) STORED
);

CREATE INDEX episodes_textidx ON episodes USING GIN (body_index);