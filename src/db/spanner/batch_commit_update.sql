UPDATE bso
   SET sortindex = COALESCE(
           (SELECT sortindex
              FROM batch_bso
             WHERE fxa_uid = @fxa_uid
               AND fxa_kid = @fxa_kid
               AND collection_id = @collection_id
               AND batch_id = @batch_id
               AND id = bso.id
            ),
            bso.sortindex
       ),

       payload = COALESCE(
           (SELECT payload
              FROM batch_bso
             WHERE fxa_uid = @fxa_uid
               AND fxa_kid = @fxa_kid
               AND collection_id = @collection_id
               AND batch_id = @batch_id
               AND id = bso.id
           ),
           bso.payload
       ),

       modified = @timestamp,

       expiry = COALESCE(
           -- TIMESTAMP_ADD returns NULL when ttl is null
           (SELECT TIMESTAMP_ADD(@timestamp, INTERVAL ttl SECOND)
              FROM batch_bso
             WHERE fxa_uid = @fxa_uid
               AND fxa_kid = @fxa_kid
               AND collection_id = @collection_id
               AND batch_id = @batch_id
               AND id = bso.id
           ),
           bso.expiry
       )
 WHERE fxa_uid = @fxa_uid
   AND fxa_kid = @fxa_kid
   AND collection_id = @collection_id
   AND id in (
       SELECT id
         FROM batch_bso
        WHERE fxa_uid = @fxa_uid
          AND fxa_kid = @fxa_kid
          AND collection_id = @collection_id
          AND batch_id = @batch_id
   )
   AND expiry > CURRENT_TIMESTAMP()
