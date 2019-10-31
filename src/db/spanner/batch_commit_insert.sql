INSERT INTO bsos (fxa_uid, fxa_kid, collection_id, bso_id, sortindex, payload, modified, expiry)
SELECT
       batch_bsos.fxa_uid,
       batch_bsos.fxa_kid,
       batch_bsos.collection_id,
       batch_bsos.batch_bso_id,

       batch_bsos.sortindex,
       COALESCE(batch_bsos.payload, ''),
       @timestamp,
       COALESCE(
           TIMESTAMP_ADD(@timestamp, INTERVAL batch_bsos.ttl SECOND),
           TIMESTAMP_ADD(@timestamp, INTERVAL @default_bso_ttl SECOND)
       )
  FROM batch_bsos
 WHERE fxa_uid = @fxa_uid
   AND fxa_kid = @fxa_kid
   AND collection_id = @collection_id
   AND batch_id = @batch_id
   AND batch_bso_id NOT in (
       SELECT bso_id
         FROM bsos
        WHERE fxa_uid = @fxa_uid
          AND fxa_kid = @fxa_kid
          AND collection_id = @collection_id
          AND expiry > CURRENT_TIMESTAMP()
   )
