FROM postgres:14.2-alpine

COPY en_us.affix  /usr/local/share/postgresql/tsearch_data/en_us.affix
COPY en_us.dict  /usr/local/share/postgresql/tsearch_data/en_us.dict
COPY schema.sql /docker-entrypoint-initdb.d/