           MERGE INTO bsos
                USING UNNEST($3) AS post
                   ON bsos.user_id = $1
                  AND bsos.collection_id = $2
                  AND bsos.bso_id = post.bso_id
    WHEN MATCHED THEN
           UPDATE SET
                      sortindex = COALESCE(post.sortindex, bsos.sortindex),
                      payload = COALESCE(post.payload, bsos.payload),
                      modified = COALESCE(
                         CASE
                         WHEN post.payload is NOT NULL OR post.sortindex IS NOT NULL THEN $4
                         ELSE NULL
                         END,
                         bsos.modified
                     ),
                      expiry = COALESCE(
                          CASE
                          WHEN post.ttl IS NOT NULL THEN $4 + (post.ttl || ' seconds')::INTERVAL
                          ELSE NULL
                          END,
                          bsos.expiry
                      )
WHEN NOT MATCHED THEN
               INSERT (user_id, collection_id, bso_id, sortindex, payload, modified, expiry)
               VALUES ($1,
                       $2,
                       post.bso_id,
                       post.sortindex,
                       COALESCE(post.payload, ''),
                       $4,
                       $4 + (COALESCE(post.ttl, $5) || ' seconds')::INTERVAL
                      )
