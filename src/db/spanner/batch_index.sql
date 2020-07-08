CREATE INDEX BatchExpireId
ON batches (
	fxa_uid,
	fxa_kid,
	collection_id,
	expiry
), INTERLEAVE IN user_collections

