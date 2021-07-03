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
    ON CONFLICT(`userid`, `collection`, `id`) DO UPDATE SET
       modified = ?,
       sortindex = COALESCE(excluded.sortindex, bso.sortindex),
       ttl = COALESCE(excluded.ttl, bso.ttl),
       payload = CASE WHEN excluded.payload != '' THEN excluded.payload ELSE bso.payload END,
       payload_size = CASE WHEN excluded.payload_size != 0 THEN excluded.payload_size ELSE bso.payload_size END
