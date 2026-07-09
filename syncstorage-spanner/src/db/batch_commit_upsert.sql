-- bsos.modified/expiry are NOT NULL with no schema default, so the insert of
-- INSERT OR UPDATE must supply a value for every such column.  Each driving
-- `batch_bsos` row LEFT JOINs to a subquery `existing` row.  On update it
-- provides values to COALESCE over; on insert the join misses, `existing.*` is
-- NULL, and the COALESCE fallback applies.  payload is nullable and mutually
-- exclusive with payload_link: whichever the batch row supplies is written and
-- the other is set NULL; when neither is supplied the existing row's values are
-- preserved.
INSERT OR UPDATE INTO bsos
    (fxa_uid, fxa_kid, collection_id, bso_id, sortindex, payload, modified, expiry,
     payload_link)
SELECT
    bb.fxa_uid,
    bb.fxa_kid,
    bb.collection_id,
    bb.batch_bso_id,
    COALESCE(bb.sortindex, existing.sortindex),
    CASE
        WHEN bb.payload IS NOT NULL THEN bb.payload
        WHEN bb.payload_link IS NOT NULL THEN NULL
        WHEN existing.payload_link IS NOT NULL THEN NULL
        ELSE COALESCE(existing.payload, '')
    END,
    @timestamp,
    COALESCE(
        TIMESTAMP_ADD(@timestamp, INTERVAL bb.ttl SECOND),
        existing.expiry,
        TIMESTAMP_ADD(@timestamp, INTERVAL @default_bso_ttl SECOND)
    ),
    CASE
        WHEN bb.payload_link IS NOT NULL THEN bb.payload_link
        WHEN bb.payload IS NOT NULL THEN NULL
        ELSE existing.payload_link
    END
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
