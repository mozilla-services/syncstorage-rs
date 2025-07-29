INSERT INTO bso (userid, collection, id, modified, sortindex, ttl, payload, payload_size)
SELECT
       ?,
       ?,
       id,
       ?,
       sortindex,
       COALESCE((ttl_offset * 1000) + ?, ?) as ttl,
       COALESCE(payload, '') as payload,
       COALESCE(payload_size, 0) as payload_size
  FROM batch_upload_items
 WHERE batch = ?
   AND userid = ?
    ON CONFLICT(userid, collection, id) DO UPDATE SET
       modified = ?,
       sortindex = COALESCE(excluded.sortindex, bso.sortindex),
       ttl = COALESCE(excluded.ttl, bso.ttl),
       payload = COALESCE(NULLIF(excluded.payload, ''), bso.payload),
       payload_size = COALESCE(excluded.payload_size, bso.payload_size)
