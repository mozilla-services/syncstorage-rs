class TestStorageMemcached(TestStorage):
    """Storage testcases run against the memcached backend, if available."""

    TEST_INI_FILE = "tests-memcached.ini"

    def setUp(self):
        # If we can't initialize due to an ImportError or BackendError,
        # assume that memcache is unavailable and skip the test.
        try:
            super(TestStorageMemcached, self).setUp()
        except (ImportError, BackendError):
            raise unittest2.SkipTest()
        except webtest.AppError as e:
            if "503" not in str(e):
                raise
            raise unittest2.SkipTest()

    # Memcache backend is configured to store tabs in cache only.
    # Add some tests the see if they still behave correctly.

    def test_strict_newer_tabs(self):
        # send two bsos in the 'tabs' collection
        bso1 = {'id': '01', 'payload': _PLD}
        bso2 = {'id': '02', 'payload': _PLD}
        bsos = [bso1, bso2]
        res = self.retry_post_json(self.root + '/storage/tabs', bsos)
        ts1 = float(res.headers["X-Last-Modified"])

        # send two more bsos
        bso3 = {'id': '03', 'payload': _PLD}
        bso4 = {'id': '04', 'payload': _PLD}
        bsos = [bso3, bso4]
        res = self.retry_post_json(self.root + '/storage/tabs', bsos)
        ts2 = float(res.headers["X-Last-Modified"])
        self.assertTrue(ts1 < ts2)

        # asking for bsos using newer=ts where newer is the timestamps
        # of bso 1 and 2, should not return them
        res = self.app.get(self.root + '/storage/tabs?newer=%s' % ts1)
        res = res.json
        res.sort()
        self.assertEquals(res, ['03', '04'])

    def test_write_tabs_503(self):
        # This can't be run against a live server.
        if self.distant:
            raise unittest2.SkipTest

        class BadCache(object):
            """Cache client stub that raises BackendError on write."""

            def __init__(self, cache):
                self.cache = cache

            def cas(self, key, *args, **kw):
                if key.endswith(":tabs"):
                    raise BackendError()
                return self.cache.cas(key, *args, **kw)

            def __getattr__(self, attr):
                return getattr(self.cache, attr)

        try:
            for key in self.config.registry:
                if key.startswith("syncstorage:storage:"):
                    storage = self.config.registry[key]
                    storage.cache = BadCache(storage.cache)

            # send two bsos in the 'tabs' collection
            bso1 = {'id': 'sure', 'payload': _PLD}
            bso2 = {'id': 'thing', 'payload': _PLD}
            bsos = [bso1, bso2]

            # we get a 503 for both POST and PUT
            self.retry_post_json(self.root + '/storage/tabs', bsos,
                                 status=503)
            self.retry_put_json(self.root + '/storage/tabs/sure', bso1,
                                status=503)
        finally:
            for key in self.config.registry:
                if key.startswith("syncstorage:storage:"):
                    storage = self.config.registry[key]
                    if isinstance(storage.cache, BadCache):
                        storage.cache = storage.cache.cache

    def test_write_tabs_ConflictError(self):
        # This can't be run against a live server.
        if self.distant:
            raise unittest2.SkipTest

        class BadCache(object):
            """Cache client stub that raises ConflictError on write."""

            def __init__(self, cache):
                self.cache = cache

            def cas(self, key, *args, **kw):
                if key.endswith(":tabs"):
                    raise ConflictError()
                return self.cache.cas(key, *args, **kw)

            def __getattr__(self, attr):
                return getattr(self.cache, attr)

        try:
            for key in self.config.registry:
                if key.startswith("syncstorage:storage:"):
                    storage = self.config.registry[key]
                    storage.cache = BadCache(storage.cache)

            # send two bsos in the 'tabs' collection
            bso1 = {'id': 'sure', 'payload': _PLD}
            bso2 = {'id': 'thing', 'payload': _PLD}
            bsos = [bso1, bso2]

            # on batch, we get back a 503
            self.retry_post_json(self.root + '/storage/tabs', bsos,
                                 status=503)

            # on single PUT, we get a 503
            self.retry_put_json(self.root + '/storage/tabs/sure', bso1,
                                status=503)
        finally:
            for key in self.config.registry:
                if key.startswith("syncstorage:storage:"):
                    storage = self.config.registry[key]
                    if isinstance(storage.cache, BadCache):
                        storage.cache = storage.cache.cache


class TestStoragePaginated(TestStorage):
    """Storage testcases run using lots of internal pagination."""

    TEST_INI_FILE = "tests-paginated.ini"


class TestStorageWithBatchUploadDisabled(TestStorage):
    """Storage testcases run with batch uploads disabled via feature flag."""

    TEST_INI_FILE = "tests-no-batch.ini"

    def test_batches(self):
        # This is the same sequence of requests as the master
        # test, but without the checks for batch semantics.
        # It lets us know that batch stuff is properly ignored.

        endpoint = self.root + '/storage/xxx_col2'

        bso1 = {'id': '12', 'payload': 'elegance'}
        bso2 = {'id': '13', 'payload': 'slovenly'}
        bsos = [bso1, bso2]
        self.retry_post_json(endpoint, bsos)

        bso3 = {'id': 'a', 'payload': 'internal'}
        bso4 = {'id': 'b', 'payload': 'pancreas'}
        resp = self.retry_post_json(endpoint + '?batch=true', [bso3, bso4])
        assert 'batch' not in resp.json
        batch = '123456'

        bso5 = {'id': 'c', 'payload': 'tinsel'}
        bso6 = {'id': '13', 'payload': 'portnoy'}
        bso0 = {'id': '14', 'payload': 'itsybitsy'}
        commit = '?batch={0}&commit=true'.format(batch)
        # This errors out because it's trying to use an in-flight batch.
        resp = self.retry_post_json(endpoint + commit, [bso5, bso6, bso0],
                                    status=400)
        # This requests a new batch, which is silently ignored and succeeds.
        commit = '?batch=true&commit=true'.format(batch)
        resp = self.retry_post_json(endpoint + commit, [bso5, bso6, bso0])
        assert 'batch' not in resp.json
        committed = resp.json['modified']
        self.assertEquals(resp.json['modified'],
                          float(resp.headers['X-Last-Modified']))

        # make sure the changes applied
        resp = self.app.get(endpoint)
        res = resp.json
        res.sort()
        self.assertEquals(res, ['12', '13', '14', 'a', 'b', 'c'])
        self.assertEquals(int(resp.headers['X-Weave-Records']), 6)
        resp = self.app.get(endpoint + '/13')
        self.assertEquals(resp.json['payload'], 'portnoy')
        self.assertEquals(committed, float(resp.headers['X-Last-Modified']))
        self.assertEquals(committed, resp.json['modified'])
        resp = self.app.get(endpoint + '/c')
        self.assertEquals(resp.json['payload'], 'tinsel')
        self.assertEquals(committed, resp.json['modified'])
        resp = self.app.get(endpoint + '/14')
        self.assertEquals(resp.json['payload'], 'itsybitsy')
        self.assertEquals(committed, resp.json['modified'])
        assert 'batch' not in resp.json

    def test_we_dont_need_no_stinkin_batches(self):
        endpoint = self.root + '/storage/xxx_col2'

        # invalid batch ID is still an error when preffed off
        bso1 = {'id': 'f', 'payload': 'pantomime'}
        self.retry_post_json(endpoint + '?batch=sammich', [bso1],
                             status=400)

        # commit with no batch ID is not an error when preffed off
        self.retry_post_json(endpoint + '?commit=true', [])

    def test_batch_partial_update(self):
        # Without batch uploads, there's nothing for this test to test.
        pass

    def test_batch_ttl_update(self):
        # Without batch uploads, there's nothing for this test to test.
        pass

    def test_batch_ttl_is_based_on_commit_timestamp(self):
        # Without batch uploads, there's nothing for this test to test.
        pass

    def test_batch_size_limits(self):
        limits = self.app.get(self.root + '/info/configuration').json
        # Without batch uploads, many config limits are not present.
        self.assertTrue('max_post_records' not in limits)
        self.assertTrue('max_post_bytes' not in limits)
        self.assertTrue('max_total_records' not in limits)
        self.assertTrue('max_total_bytes' not in limits)
        self.assertTrue('max_record_payload_bytes' in limits)

    def test_batch_with_failing_bsos(self):
        # Without batch uploads, there's nothing for this test to test.
        pass

    def test_batch_id_is_correctly_scoped_to_a_user(self):
        # Without batch uploads, there's nothing for this test to test.
        pass

    def test_batch_id_is_correctly_scoped_to_a_collection(self):
        # Without batch uploads, there's nothing for this test to test.
        pass

    def test_users_with_the_same_batch_id_get_separate_data(self):
        # Without batch uploads, there's nothing for this test to test.
        pass

    def test_that_we_dont_resurrect_committed_batches(self):
        # Without batch uploads, there's nothing for this test to test.
        pass

    def test_batch_empty_commit(self):
        # Without batch uploads, there's nothing for this test to test.
        pass


class TestStorageMemcachedWriteThrough(TestStorageMemcached):
    """Storage testcases run against the memcached backend, if available.

    These tests are configured to use the write-through cache for all the
    test-related collections.
    """

    TEST_INI_FILE = "tests-memcached-writethrough.ini"

    def test_write_tabs_ConflictError(self):
        # ConflictErrors in the cache are ignored in write-through mode,
        # since it can just lazily re-populate from the db.
        pass


class TestStorageMemcachedCacheOnly(TestStorageMemcached):
    """Storage testcases run against the memcached backend, if available.

    These tests are configured to use the cache-only-collection behaviour
    for all the test-related collections.
    """

    TEST_INI_FILE = "tests-memcached-cacheonly.ini"
