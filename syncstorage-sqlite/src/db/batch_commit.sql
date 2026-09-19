INSERT INTO bso (userid, collection, id, modified, sortindex, ttl, payload, payload_size)
SELECT
       ?,
       ?,
       batch_upload_items.id,
       ?,
       batch_upload_items.sortindex,
       COALESCE((batch_upload_items.ttl_offset * 1000) + ?, bso.ttl, ?),
       COALESCE(batch_upload_items.payload, bso.payload, ''),
       COALESCE(batch_upload_items.payload_size, bso.payload_size, 0)
  FROM batch_upload_items
  LEFT JOIN bso
    ON bso.userid = ?
   AND bso.collection = ?
   AND bso.id = batch_upload_items.id
 WHERE batch_upload_items.batch = ?
   AND batch_upload_items.userid = ?
    ON CONFLICT(userid, collection, id) DO UPDATE SET
       modified = excluded.modified,
       sortindex = COALESCE(excluded.sortindex, sortindex),
       ttl = excluded.ttl,
       payload = excluded.payload,
       payload_size = excluded.payload_size
