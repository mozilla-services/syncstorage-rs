INSERT INTO bso (userid, collection, id, modified, sortindex, ttl, payload, payload_size)
SELECT
       ?,
       ?,
       id,
       ?,
       sortindex,
       COALESCE((ttl_offset * 1000) + ?, ?),
       COALESCE(payload, ''),
       COALESCE(payload_size, 0)
  FROM batch_upload_items
 WHERE batch = ?
   AND userid = ?
    ON DUPLICATE KEY UPDATE
       modified = ?,
       sortindex = COALESCE(batch_upload_items.sortindex, bso.sortindex),
       ttl = COALESCE((batch_upload_items.ttl_offset * 1000) + ?, bso.ttl),
       payload = COALESCE(batch_upload_items.payload, bso.payload),
       payload_size = COALESCE(batch_upload_items.payload_size, bso.payload_size)
