-- bsos.payload/modified/expiry are NOT NULL with no schema default, so the
-- insert of INSERT OR UPDATE must supply a value for every column.  Each
-- driving `batch_bsos` row LEFT JOINs to a subquery `existing` row.  On update
-- it provides values to COALESCE over; on insert the join misses, `existing.*`
-- is NULL, and the COALESCE fallback applies.
INSERT OR UPDATE INTO bsos
    (fxa_uid, fxa_kid, collection_id, bso_id, sortindex, payload, modified, expiry,
     payload_link)
SELECT
    bb.fxa_uid,
    bb.fxa_kid,
    bb.collection_id,
    bb.batch_bso_id,
    COALESCE(bb.sortindex, existing.sortindex),
    COALESCE(bb.payload, existing.payload, ''),
    @timestamp,
    COALESCE(
        TIMESTAMP_ADD(@timestamp, INTERVAL bb.ttl SECOND),
        existing.expiry,
        TIMESTAMP_ADD(@timestamp, INTERVAL @default_bso_ttl SECOND)
    ),
    COALESCE(bb.payload_link, existing.payload_link)
  FROM batch_bsos AS bb
  LEFT JOIN (
      SELECT fxa_uid, fxa_kid, collection_id, bso_id,
             sortindex, payload, expiry, payload_link
        FROM bsos
       WHERE fxa_uid = @fxa_uid
         AND fxa_kid = @fxa_kid
         AND collection_id = @collection_id
  ) AS existing
    ON existing.fxa_uid = bb.fxa_uid
   AND existing.fxa_kid = bb.fxa_kid
   AND existing.collection_id = bb.collection_id
   AND existing.bso_id = bb.batch_bso_id
 WHERE bb.fxa_uid = @fxa_uid
   AND bb.fxa_kid = @fxa_kid
   AND bb.collection_id = @collection_id
   AND bb.batch_id = @batch_id
