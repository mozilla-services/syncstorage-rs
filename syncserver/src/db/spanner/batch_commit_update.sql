UPDATE bsos
   SET sortindex = COALESCE(
           (SELECT sortindex
              FROM batch_bsos
             WHERE fxa_uid = @fxa_uid
               AND fxa_kid = @fxa_kid
               AND collection_id = @collection_id
               AND batch_id = @batch_id
               AND batch_bso_id = bsos.bso_id
            ),
            bsos.sortindex
       ),

       payload = COALESCE(
           (SELECT payload
              FROM batch_bsos
             WHERE fxa_uid = @fxa_uid
               AND fxa_kid = @fxa_kid
               AND collection_id = @collection_id
               AND batch_id = @batch_id
               AND batch_bso_id = bsos.bso_id
           ),
           bsos.payload
       ),

       modified = @timestamp,

       expiry = COALESCE(
           -- TIMESTAMP_ADD returns NULL when ttl is null
           (SELECT TIMESTAMP_ADD(@timestamp, INTERVAL ttl SECOND)
              FROM batch_bsos
             WHERE fxa_uid = @fxa_uid
               AND fxa_kid = @fxa_kid
               AND collection_id = @collection_id
               AND batch_id = @batch_id
               AND batch_bso_id = bsos.bso_id
           ),
           bsos.expiry
       )
 WHERE fxa_uid = @fxa_uid
   AND fxa_kid = @fxa_kid
   AND collection_id = @collection_id
   AND bso_id in (
       SELECT batch_bso_id
         FROM batch_bsos
        WHERE fxa_uid = @fxa_uid
          AND fxa_kid = @fxa_kid
          AND collection_id = @collection_id
          AND batch_id = @batch_id
   )
