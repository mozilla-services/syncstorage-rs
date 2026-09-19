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
    ON CONFLICT(userid, collection, id) DO UPDATE SET
       modified = excluded.modified,
       sortindex = COALESCE(excluded.sortindex, sortindex),
       ttl = COALESCE(excluded.ttl, ttl),
       payload = COALESCE(excluded.payload, payload),
       payload_size = COALESCE(excluded.payload_size, payload_size)
