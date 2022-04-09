import os.path
import psycopg2
import sys
import json

conn = psycopg2.connect(sys.argv[1])
cur = conn.cursor()

with open("seasons-metadata.json") as seasonsf:
    seasons = json.load(seasonsf)

for season in seasons:
    for ep in season["episodes"]:
        with open(
            os.path.join("transcripts", season["slug"], ep["id"] + ".txt")
        ) as episodef:
            episode_body = episodef.read()

        params = {
            "id": ep["id"],
            "title": ep["title"],
            "season": season["slug"],
            "body": episode_body,
        }

        cur.execute(
            "INSERT INTO episodes (id, season, title, body) VALUES (%(id)s, %(season)s, %(title)s, %(body)s) ON CONFLICT(id) DO UPDATE SET body = %(body)s",
            params,
        )

conn.commit()
cur.close()
conn.close()
