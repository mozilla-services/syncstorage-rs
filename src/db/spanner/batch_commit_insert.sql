INSERT INTO bso (fxa_uid, fxa_kid, collection_id, id, sortindex, payload, modified, expiry)
SELECT
       batch_bso.fxa_uid,
       batch_bso.fxa_kid,
       batch_bso.collection_id,
       batch_bso.id,

       batch_bso.sortindex,
       COALESCE(batch_bso.payload, ''),
       @timestamp,
       COALESCE(
           TIMESTAMP_ADD(@timestamp, INTERVAL batch_bso.ttl SECOND),
           TIMESTAMP_ADD(@timestamp, INTERVAL @default_bso_ttl SECOND)
       )
  FROM batch_bso
 WHERE fxa_uid = @fxa_uid
   AND fxa_kid = @fxa_kid
   AND collection_id = @collection_id
   AND batch_id = @batch_id
   AND id NOT in (
       SELECT id
         FROM bso
        WHERE fxa_uid = @fxa_uid
          AND fxa_kid = @fxa_kid
          AND collection_id = @collection_id
          AND expiry > CURRENT_TIMESTAMP()
   )
