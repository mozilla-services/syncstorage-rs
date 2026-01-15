           MERGE INTO bsos
                USING (
                      SELECT batch_bso_id, sortindex, payload, ttl
                        FROM batch_bsos
                       WHERE user_id = $1
                         AND collection_id = $2
                         AND batch_id = $3
                      ) AS batch
                   ON bsos.user_id = $1
                  AND bsos.collection_id = $2
                  AND bsos.bso_id = batch.batch_bso_id
    WHEN MATCHED THEN
           UPDATE SET
                      sortindex = COALESCE(batch.sortindex, bsos.sortindex),
                      payload = COALESCE(batch.payload, bsos.payload),
                      modified = $4,
                      expiry = COALESCE(
                          CASE
                          WHEN batch.ttl IS NOT NULL THEN $4 + (batch.ttl || ' seconds')::INTERVAL
                          ELSE NULL
                          END,
                          bsos.expiry
                      )
WHEN NOT MATCHED THEN
               INSERT (user_id, collection_id, bso_id, sortindex, payload, modified, expiry)
               VALUES ($1,
                       $2,
                       batch.batch_bso_id,
                       batch.sortindex,
                       COALESCE(batch.payload, ''),
                       $4,
                       $4 + (COALESCE(batch.ttl, $5) || ' seconds')::INTERVAL
                      )
