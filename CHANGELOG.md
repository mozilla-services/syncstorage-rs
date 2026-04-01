<a name="0.22.1"></a>
## 0.22.1 (2026-04-01)


#### Chore

*   capture sentry backtraces (#2166) ([f0497396](https://github.com/mozilla-services/syncstorage-rs/commit/f049739658ab5ce46411800672042f0e448effcd))
*   remove HTTP status metric from syncstorage-rs (#2163) ([19847721](https://github.com/mozilla-services/syncstorage-rs/commit/19847721577b49ec911433194a8b74a4a35473cb))
*   build and push "directly" instead of using mozilla-it/deploy-actions (#2149) ([36e5683a](https://github.com/mozilla-services/syncstorage-rs/commit/36e5683a62c9355348fc3ef10877f1c91664c79e))
*   upgrade Python for all utils and refactor (#2127) ([6b09e994](https://github.com/mozilla-services/syncstorage-rs/commit/6b09e9947a57b1d6dc816f2905b45d2184326ac8))
*   bump rustls-webpki per RUSTSEC-2026-0049 (#2150) ([db5834db](https://github.com/mozilla-services/syncstorage-rs/commit/db5834db64729bba5431ae2c83bb78351a8b63fe))
*   resolve aws-lc-rs vuln (#2148) ([cd18eae5](https://github.com/mozilla-services/syncstorage-rs/commit/cd18eae541357f91532fb385f291c28d64fa83f7))

#### Features

*   also push a latest tag to ent GAR (#2182) ([5513da89](https://github.com/mozilla-services/syncstorage-rs/commit/5513da89cdcd4fda0bc82d6ad917e2e65587396a))
*   single workflow and actions optimizations (#2140) ([cf1d30ff](https://github.com/mozilla-services/syncstorage-rs/commit/cf1d30ffb723c41e128a44dd57e9c586faef7cf7))
*   (re-)enable timestamp+offset based pagination optimization (#2145) ([c1d53b60](https://github.com/mozilla-services/syncstorage-rs/commit/c1d53b604f099e5fd43fde81143031c4d8cc4705))
*   add logging for acct webhook handler (#2147) ([ecb6bd05](https://github.com/mozilla-services/syncstorage-rs/commit/ecb6bd05dd9379748e4c65dd6f9b6640e9c710d9))
* **mozcloud-publish:**  updated mozcloud-publish workflow to trigger on tokenserver-preview labels and consolidated checks into a job that is required by all build jobs (#2135) ([685075e7](https://github.com/mozilla-services/syncstorage-rs/commit/685075e760d856bed84ed44be10d8cb8d4b202ca))

#### Refactor

*   stop passing collection id to get_quota_usage (#2170) ([857739dd](https://github.com/mozilla-services/syncstorage-rs/commit/857739dd4ce3e8344d5cfa9b0628712e264c2290))

#### Doc

*   fixes syncserver PostgreSQL GHCR image name (#2153) ([5dbe6bc4](https://github.com/mozilla-services/syncstorage-rs/commit/5dbe6bc4ea20733690def17aa4acbed0ff560738))
*   fix broken link to config page (#2144) ([ed6e27fb](https://github.com/mozilla-services/syncstorage-rs/commit/ed6e27fbb7a2d616aa7b682315a835ed1fbc4d1d))



<a name="0.22.0"></a>
## 0.22.0 (2026-03-17)


#### Test

*   molotov sync loadtests (#2052) ([408a23fe](https://github.com/mozilla-services/syncstorage-rs/commit/408a23fee81fca52e52d69408c8cf8660100769c))
*   rollback transaction in tests relying on Spanner emulator (#2045) ([6c6c0ffd](https://github.com/mozilla-services/syncstorage-rs/commit/6c6c0ffd1db1218a64ee5e058f205c44a14be8bc))
*   fix flake in test_users_with_the_same_batch_id_get_separate_data (#1981) ([01edad76](https://github.com/mozilla-services/syncstorage-rs/commit/01edad768e99d13bf852c54002adf2bf145010e4))

#### Breaking Changes

*   switch to TIMESTAMPTZ (#1932) ([5c022c04](https://github.com/mozilla-services/syncstorage-rs/commit/5c022c04dfa9c643888538e531b5a81a35ca0792), breaks [#](https://github.com/mozilla-services/syncstorage-rs/issues/))

#### Bug Fixes

*   set new node's available = capacity (#2111) ([33edb814](https://github.com/mozilla-services/syncstorage-rs/commit/33edb8146991f78b6f406d9f9b92339414ba1b4c))
*   imeplement security guidelines for github actions (#2096) ([47fe53e4](https://github.com/mozilla-services/syncstorage-rs/commit/47fe53e44a2b1401d269e6fe14167cb7ba6bf95e))
*   adjust tokenserver scripts per postgres (#2086) ([7b765be5](https://github.com/mozilla-services/syncstorage-rs/commit/7b765be5d8994561fcfed9cd61c4cee8ab28d97a))
*   re-add tokenserver/spanner scripts to the final docker (#2083) ([f2b71995](https://github.com/mozilla-services/syncstorage-rs/commit/f2b7199519f0a5be76a0effe9b5b8bfd59a40675))
*   get_storage_timestamp/lock_for_read should ignore PRETOUCH_DT (#2067) ([0d780c74](https://github.com/mozilla-services/syncstorage-rs/commit/0d780c74cf496ba1f74a324253eda06c968eec1a))
*   move version.json to the root (#2041) ([e8a392b0](https://github.com/mozilla-services/syncstorage-rs/commit/e8a392b0a38a0a085b560804842b98638b0c5c01))
*   correct the JWK env var examples/docs (#2022) ([d26ca214](https://github.com/mozilla-services/syncstorage-rs/commit/d26ca2146ebe2db0b38ed8923135bf4d5c212be0))
*   preserve the uid ordering by sorting in reverse (#2017) ([574f3552](https://github.com/mozilla-services/syncstorage-rs/commit/574f3552d6352e6fe45d11b1062f5069118f4a38))
*   further downgrade mermaid, internal dep env problem (#1974) ([a3ca41fe](https://github.com/mozilla-services/syncstorage-rs/commit/a3ca41fe42c0966b1937012fa825b6d7333b2528))
*   update workflow not use script (#1972) ([38fce2f3](https://github.com/mozilla-services/syncstorage-rs/commit/38fce2f3f46eb1a3d10350d98e3f6e03ae109aa8))
*   point to tokenserver api (#1970) ([d05cf818](https://github.com/mozilla-services/syncstorage-rs/commit/d05cf81858c9fbc83b623eafd466fe76a4a84dc1))
*   address reserve and custom collections (#1950) ([8b15ac53](https://github.com/mozilla-services/syncstorage-rs/commit/8b15ac53deea27d3cb405b8d31c4fce2fc7a4861))
*   preserve existing value on batch append if new value is null/empty (#1943) ([424ab1e7](https://github.com/mozilla-services/syncstorage-rs/commit/424ab1e7c9b807e93e058f8afb545c3e678a8b89))
*   ensure Postgres return types and Rust value types match (#1940) ([a94d702c](https://github.com/mozilla-services/syncstorage-rs/commit/a94d702ce773187058e60979fd5bf390083df64c))
*   initializing the app once suffices (#1937) ([afaafc57](https://github.com/mozilla-services/syncstorage-rs/commit/afaafc570471d713e40865e691a9b4bd4f5ffece))
*   set collection id col to auto-incr and fix get_collection_id (#1929) ([9bc614d7](https://github.com/mozilla-services/syncstorage-rs/commit/9bc614d7b9a40beb6e5c03e6dcbd6a8f597aff67))
*   mariadb compatibility ([b1ca7b32](https://github.com/mozilla-services/syncstorage-rs/commit/b1ca7b32deeb4b5a6d32d875ebfbc8b5e2924629), closes [#1753](https://github.com/mozilla-services/syncstorage-rs/issues/1753))
* **tokenserver:**  use actual postgres post_user in release build ([d5de9b14](https://github.com/mozilla-services/syncstorage-rs/commit/d5de9b149b924a7c45edb2c4f6fa912cdbab5248))

#### Refactor

*   resolve vulns and upgrade (#1788) ([88e7eb9c](https://github.com/mozilla-services/syncstorage-rs/commit/88e7eb9ca53429d6e85f97138043e11d2a8f70b2))
*   extract a PgDb::check_quota method ([4b399232](https://github.com/mozilla-services/syncstorage-rs/commit/4b3992327eec6fe82be47dc2c80882f946ded243))
*   use u64 for DEFAULT_MAX_QUOTA_LIMIT ([f8d63999](https://github.com/mozilla-services/syncstorage-rs/commit/f8d63999f43f19f8a6c2c8aaec826a700a6b1697))
*   divvy up the db module ([30bb0556](https://github.com/mozilla-services/syncstorage-rs/commit/30bb0556f61f9c469686cc2a2ce058d27f8cc6ba))
*   divvy up tokenserver-postgres's db module ([eea08fb2](https://github.com/mozilla-services/syncstorage-rs/commit/eea08fb2cd00747d4bc8af3b0eafffb246447bac))
*   tokenserver-postgres models/orm_models/schema -> db ([65becde0](https://github.com/mozilla-services/syncstorage-rs/commit/65becde05a16da41d1c9b8f6f98547cdd1aaa378))
*   divvy up tokenserver-mysql's db module ([51e487d1](https://github.com/mozilla-services/syncstorage-rs/commit/51e487d1db0942a42489d018f10d23201a8b7da7))
*   tokenserver-mysql models/pool/migrations -> db ([992b1908](https://github.com/mozilla-services/syncstorage-rs/commit/992b190861df11211f7af0a77590301a72ac410d))
*   divvy up the db/batch impls into their own mods ([7ecab8bb](https://github.com/mozilla-services/syncstorage-rs/commit/7ecab8bb3967ab7f85352e58c286b4bfa71ebdfe))
*   models/batch/schema/diesel_ext -> db ([4145a469](https://github.com/mozilla-services/syncstorage-rs/commit/4145a469f8eb34686723be946d53edec44b040c2))
*   move syncstorage-mysql's error -> db-common ([e8d89d7a](https://github.com/mozilla-services/syncstorage-rs/commit/e8d89d7ab610207d47b1c9ae8f3329e87113376c))
*   separate batch calls into a BatchDb trait ([01566137](https://github.com/mozilla-services/syncstorage-rs/commit/015661378191029eb50a990bde4c4ab5cc98711d))
*   stream/support/BATCH_COMMIT.txt -> db ([da27f9de](https://github.com/mozilla-services/syncstorage-rs/commit/da27f9defbdd4ea736bf1ae00de52a3e2d4d35e6))
*   String -> &str ([21bcc42f](https://github.com/mozilla-services/syncstorage-rs/commit/21bcc42febb15e056a354225af96b47b36468401))
*   divvy up the db impl into its own mod ([84de3272](https://github.com/mozilla-services/syncstorage-rs/commit/84de3272018be92c3320c719bab9c5609855bd53))
*   models/batch -> db ([80358fe6](https://github.com/mozilla-services/syncstorage-rs/commit/80358fe69c72e9e6cbb47696f1650ac2b55c4da9))
*   rmv last insert id, simplify queries, add check (#1841) ([8e3f3670](https://github.com/mozilla-services/syncstorage-rs/commit/8e3f3670950979a1fd9494bcdcd2258b61272249))
*   move syncstorage's Db to async-trait (#1830) ([d32c669b](https://github.com/mozilla-services/syncstorage-rs/commit/d32c669b7ae5af88e50e8dd9711358d7699e44b6))
*   kill SpannerDb's RefCell (#1826) ([613f6ed3](https://github.com/mozilla-services/syncstorage-rs/commit/613f6ed37ef71988aa6ea680110027c19420b4cd))
*   kill r2d2 references, share From<PoolError> ([bcf425c6](https://github.com/mozilla-services/syncstorage-rs/commit/bcf425c635405b720268429d724fdd6271d4ca86))
*   move shared tokenserver db into tokenserver-db-common (#1801) ([40b21345](https://github.com/mozilla-services/syncstorage-rs/commit/40b21345fa956ed6cb2502b33f74a8eb7f19f8c6))
* **db:**  return only a SyncTimestamp from post_bsos db fns ([730d6b81](https://github.com/mozilla-services/syncstorage-rs/commit/730d6b8173f636381038b2b72a30a94daadda8bb))
* **metrics:**  make hostname in metrics optional (#1880) ([9e0d3698](https://github.com/mozilla-services/syncstorage-rs/commit/9e0d3698518f76939e6af8718a3ad532c1aaf454))

#### Features

*   adapt PoolState to usage of deadpool everywhere (#2128) ([019bf46a](https://github.com/mozilla-services/syncstorage-rs/commit/019bf46adfeb30d83c4da3c14ae67c60bafb7b9d))
*   add FxA event webhook endpoint (#2108) ([efa8c4b7](https://github.com/mozilla-services/syncstorage-rs/commit/efa8c4b759a8c1cd7e02b29e1cef0198169f050d))
*   normalize use of `chrono` for time and datetime operations (#2125) ([cdd0c3da](https://github.com/mozilla-services/syncstorage-rs/commit/cdd0c3dad5d36d1a8b883c7c7f540bf35a558fb1))
*   log when initializing a nodes table entry (#2104) ([f2e18e2d](https://github.com/mozilla-services/syncstorage-rs/commit/f2e18e2df9e166a66d00ecf41b89806b905b86d6))
*   push postgres dockers to enterprise gar (#2100) ([2823c2e8](https://github.com/mozilla-services/syncstorage-rs/commit/2823c2e821a4a779dda798fce3fcda1a90f601a7))
*   upsert the first storage node record with env var (#2087) ([0f759c6f](https://github.com/mozilla-services/syncstorage-rs/commit/0f759c6fc666c3dcdf113c3445a777cff50395dc))
*   log when Sentry is not configured (#2073) ([66d9d11e](https://github.com/mozilla-services/syncstorage-rs/commit/66d9d11e27f98ef9c3526ee6ae380cf6bc092694))
*   postgres node type (#2076) ([50a739b5](https://github.com/mozilla-services/syncstorage-rs/commit/50a739b58dc9ec81995f86e71d992aa14ccc450e))
*   stick w/ Continuous Delivery style image tags (#2044) ([9ac27a37](https://github.com/mozilla-services/syncstorage-rs/commit/9ac27a378ff0be2ba06bd1e6d0df8db6b4614dd6))
*   optimize postgres get_or_create_collection_id (#2028) ([935b468c](https://github.com/mozilla-services/syncstorage-rs/commit/935b468cb07756603ce4b4ad83e7352f2c56593f))
*   migrate docker hub push to github actions (#2016) ([efb70a1f](https://github.com/mozilla-services/syncstorage-rs/commit/efb70a1f2929e90d96ea3b768c58cb3d568b39ff))
*   migrate to GitHub Actions for Spanner build and tests (#2015) ([25c852a3](https://github.com/mozilla-services/syncstorage-rs/commit/25c852a33f62b4ad19385064136343ab6242f49c))
*   optimize post_bsos w/ MERGE INTO ([eff8f805](https://github.com/mozilla-services/syncstorage-rs/commit/eff8f805d46b2b0d109de8b599e2c3961e1ad622))
*   migrate code checks to github actions (#2005) ([3cca75a9](https://github.com/mozilla-services/syncstorage-rs/commit/3cca75a9bac9d715652efc14d1b2a2d59bced5b8))
*   kill long removed browserid references ([ab1887d2](https://github.com/mozilla-services/syncstorage-rs/commit/ab1887d2838a57b4182de3ac7429f1a2ce72e08c))
*   optimize batch_commit w/ MERGE INTO (#2003) ([9ed0d0c2](https://github.com/mozilla-services/syncstorage-rs/commit/9ed0d0c2cd8e3a043d492edd4e34a68b77e32b58))
*   bootstrap the sync-1.5 service table entry (#1993) ([0bc8c98d](https://github.com/mozilla-services/syncstorage-rs/commit/0bc8c98d9be256e268e545f108a3beec4703f6cb))
*   get the timestamp from postgres (#1962) ([0cca7c73](https://github.com/mozilla-services/syncstorage-rs/commit/0cca7c7338b14cab4e185e2bfd703a9ad34eb8ce))
*   emit unexpected verify.py exceptions to sentry (#1951) ([9fbeaee2](https://github.com/mozilla-services/syncstorage-rs/commit/9fbeaee2062477630907afbac5f49e8ca5f1b87d))
*   re-enable warnings (#1947) ([daec1917](https://github.com/mozilla-services/syncstorage-rs/commit/daec191739e73323e530160befbb6c1c96693dfa))
*   postgres docker and gar (#1945) ([d4a3c24b](https://github.com/mozilla-services/syncstorage-rs/commit/d4a3c24b0cb79056c98cceab091ee164f113b7f8))
*   switch to TIMESTAMPTZ (#1932) ([5c022c04](https://github.com/mozilla-services/syncstorage-rs/commit/5c022c04dfa9c643888538e531b5a81a35ca0792), breaks [#](https://github.com/mozilla-services/syncstorage-rs/issues/))
*   UserIdentifier modification in lock and timestamp methods (#1927) ([56c21e0f](https://github.com/mozilla-services/syncstorage-rs/commit/56c21e0fef9a6464257a822d9910813d37cbfede))
*   support query logging via diesel's instrumentation (#1925) ([cb8e620a](https://github.com/mozilla-services/syncstorage-rs/commit/cb8e620a3997776105e86ee897113d43336276f8))
*   create post/put bsos postgres (#1911) ([855066ce](https://github.com/mozilla-services/syncstorage-rs/commit/855066ced400ff25c98b0fe496bcc6cb24491d2d))
*   impl {create,commit,append_to,get}_batch for Postgres ([4208f037](https://github.com/mozilla-services/syncstorage-rs/commit/4208f037f1b3defd2bb2d5113314aec1eed8d6aa))
*   quota methods (#1920) ([278ffcc7](https://github.com/mozilla-services/syncstorage-rs/commit/278ffcc74a7fbc81260e73bb07487ccdc5d06b45))
*   add the remaining postgres get methods (#1912) ([1edfd870](https://github.com/mozilla-services/syncstorage-rs/commit/1edfd870b3f6a27a9e2b2b633b3b2d556d606de7))
*   add delete fns for Postgres db impls ([f548ea1f](https://github.com/mozilla-services/syncstorage-rs/commit/f548ea1fc3ed985774c78d8fe175d4e26d395563))
*   impl for update collection (#1900) ([6b0b41c6](https://github.com/mozilla-services/syncstorage-rs/commit/6b0b41c6d5ac63c55ddef03194558941019967eb))
*   postgres read and write locks (#1891) ([4d6fe26a](https://github.com/mozilla-services/syncstorage-rs/commit/4d6fe26a3553c4ea11fe48b6bcf57b6611d81f7f))
*   add get_bsos/bso_ids (#1899) ([f3a33250](https://github.com/mozilla-services/syncstorage-rs/commit/f3a33250b6a11f088e9cd152206085f239931b50))
*   support systemd-journal logging (#1858) ([7ba0b433](https://github.com/mozilla-services/syncstorage-rs/commit/7ba0b4338f62a250d3591901187ab83e02906346))
*   load and map collections impl (#1898) ([455ad7ae](https://github.com/mozilla-services/syncstorage-rs/commit/455ad7aed2f0ef4df64ecafe8d6e0510fad72d6e))
*   add script to purge expired items in Postgres ([845a03b2](https://github.com/mozilla-services/syncstorage-rs/commit/845a03b2a5385c5e63530099fb0a9192b7fcbedf))
*   impl get collection and collection id methods (#1887) ([ac10f5a7](https://github.com/mozilla-services/syncstorage-rs/commit/ac10f5a7c2d61445043d549dbbdeacb704ee5e3e))
*   postgres db pool and session implementation (#1875) ([a65d4610](https://github.com/mozilla-services/syncstorage-rs/commit/a65d461094486a42fd49dd91a7e3e550f2aad6a5))
*   create schema and orm models for sync postgres (#1873) ([7fa191e6](https://github.com/mozilla-services/syncstorage-rs/commit/7fa191e6995402d63b25ac02fb386964d9629556))
*   sync postgres schema (#1853) ([0c119849](https://github.com/mozilla-services/syncstorage-rs/commit/0c1198496d8f2d5474d28924bdc59a42a0095000))
*   tokenserver script postgres support (#1857) ([adbb0a65](https://github.com/mozilla-services/syncstorage-rs/commit/adbb0a65d993c1b96bf2744751d5fa3c47b9536e))
*   add syncstorage-postgres ([bcc5990e](https://github.com/mozilla-services/syncstorage-rs/commit/bcc5990eea890cce250710a5a782b289511a8816))
*   postgres user methods (#1839) ([d6e5f736](https://github.com/mozilla-services/syncstorage-rs/commit/d6e5f7360eecd677cee70d5fd311c7cdf9399e1d))
*   postgres node methods (#1828) ([d7e737dd](https://github.com/mozilla-services/syncstorage-rs/commit/d7e737ddab2e4b80122d764a7614242eb772d463))
*   postgres service methods  (#1814) ([daec270a](https://github.com/mozilla-services/syncstorage-rs/commit/daec270abae2d1fbded8dfe57c7262c16961c12c))
*   switch syncstorage to diesel-async ([192d64c7](https://github.com/mozilla-services/syncstorage-rs/commit/192d64c7c345a3ae4945432f0c89d5d8205c3960))
*   adapt the test suite to the generic dyn DbPool (#1808) ([0400c2dd](https://github.com/mozilla-services/syncstorage-rs/commit/0400c2dd1547b26a21d51d3ea0d3398501684686))
*   create tokenserver postgres db trait (#1809) ([8d8b79e1](https://github.com/mozilla-services/syncstorage-rs/commit/8d8b79e1b5849aa27d4eb4f57452f635012decc8))
*   create tokenserver postgres db pool (#1806) ([0c3c06d5](https://github.com/mozilla-services/syncstorage-rs/commit/0c3c06d50eb62e6a150eabeebfc8c25b178630aa))

#### Chore

*   rmv oauth tokenserver auth method (#2139) ([23d04224](https://github.com/mozilla-services/syncstorage-rs/commit/23d04224e1e916c3a8800552dc9dae7ad045cf36))
*   update test result/coverage filenames (#2113) ([37350249](https://github.com/mozilla-services/syncstorage-rs/commit/37350249e89b701f13c930b4a460bb583b9549e9))
*   actions security updates (#2109) ([f2e6cf5e](https://github.com/mozilla-services/syncstorage-rs/commit/f2e6cf5e274586ac62d781fac43d87da73fe4a8f))
*   run gh workflows on pushes (#2105) ([a66dfa71](https://github.com/mozilla-services/syncstorage-rs/commit/a66dfa71cce1ea7a941828cd491491ed36f3bdd8))
*   rm .circleci (#2101) ([8f289f3a](https://github.com/mozilla-services/syncstorage-rs/commit/8f289f3a5d8377c2e28c15ce06f1cc65bcc5aa7c))
*   upgrade jsonwebtoken (#2097) ([9db210d5](https://github.com/mozilla-services/syncstorage-rs/commit/9db210d5a526aaa677f8a3cc844b4f7191f8911b))
*   stop uploading test results to gcs from circleci (#2094) ([c9fd339d](https://github.com/mozilla-services/syncstorage-rs/commit/c9fd339df101e07dfbae02f5f66a39d705a269d3))
*   build test results file prefix when github (#2092) ([dc07e833](https://github.com/mozilla-services/syncstorage-rs/commit/dc07e833074d14d88088599e1a3271164dc65bde))
*   stop checking for secrets.ETE_GCLOUD_SERVICE_KEY in workflows (#2090) ([129c1aa0](https://github.com/mozilla-services/syncstorage-rs/commit/129c1aa0a549acc222af0aae6e888c990a066280))
*   update rust to 1.91 (#2082) ([68081b2b](https://github.com/mozilla-services/syncstorage-rs/commit/68081b2b7eef5e776ecb29e939ac8196fb81342f))
*   `cargo upgrade` deps (#2061) ([b54129a3](https://github.com/mozilla-services/syncstorage-rs/commit/b54129a334097159802bdd9b06f6961508379ec2))
*   `poetry update` python deps (#2063) ([33d41fbf](https://github.com/mozilla-services/syncstorage-rs/commit/33d41fbf3b8666455907d506237360009bcded2a))
*   update image names and jobs to syncserver (#2051) ([f545cb3e](https://github.com/mozilla-services/syncstorage-rs/commit/f545cb3eb66a2f24047e53dfdb4b946ccc635193))
*   mv load tests (#2060) ([ab47abaa](https://github.com/mozilla-services/syncstorage-rs/commit/ab47abaaeda7e91ff423168043e99560a72a9891))
*   remove pushes to dockerhub (#2057) ([cfb3e5c3](https://github.com/mozilla-services/syncstorage-rs/commit/cfb3e5c304abd94045aea2a006ab05ddbb05793c))
*   upgrade to Rust 2024 edition (#2048) ([0ddc3b0a](https://github.com/mozilla-services/syncstorage-rs/commit/0ddc3b0af158c4bba2758229c941330745a18425))
*   use dorny/test-reporter for reporting gh action test results (#2036) ([9f700d64](https://github.com/mozilla-services/syncstorage-rs/commit/9f700d642e5bd21ad40bce552bf4eedd5ea17118))
*   Update GAR Image Tagging to MozCloud Spec (#2029) ([d2fda79d](https://github.com/mozilla-services/syncstorage-rs/commit/d2fda79d955897177ea4bc5f79798f135d27993f))
*   migrate to GH action for Postgres build and tests (#2012) ([6af6d5c6](https://github.com/mozilla-services/syncstorage-rs/commit/6af6d5c6b6b58edc5eb272926000ec93dbb7f2e1))
*   move MySQL build and test to GH actions (#2011) ([f125cf6b](https://github.com/mozilla-services/syncstorage-rs/commit/f125cf6b32a310bab07b0b86d57d4be16607ce5b))
*   kill remaining requirements.txt ([5f8ab548](https://github.com/mozilla-services/syncstorage-rs/commit/5f8ab5483fda183417097a69ae3ba62d9dbfb1fe))
*   clean out OnDuplicateKeyUpdate mysql diesel extension (#2001) ([ac614115](https://github.com/mozilla-services/syncstorage-rs/commit/ac614115f7277acaf5b199f4beb9fd452548be0f))
*   build image for postgres python utils (#1987) ([e7b4ccdf](https://github.com/mozilla-services/syncstorage-rs/commit/e7b4ccdff3e0a3d2f512050ab6a5b0be44e9617c))
*   bump mbdook to 0.5.2 (#1988) ([ba6cda12](https://github.com/mozilla-services/syncstorage-rs/commit/ba6cda12ca8f6bf7e24a0aeeb81320accd038081))
*   update mdbook and mermaid (#1977) ([1c12ca06](https://github.com/mozilla-services/syncstorage-rs/commit/1c12ca069500a68e63bf9f7bb4fff0f1705288a7))
*   doc workflow fix (#1967) ([6fca1c40](https://github.com/mozilla-services/syncstorage-rs/commit/6fca1c40225906298ba3f676a069f46b91fe6b02))
*   use real exit codes for Postgres tests in CI (#1944) ([7bf49d79](https://github.com/mozilla-services/syncstorage-rs/commit/7bf49d79a7dab06650ead4c4c49a0ba9c62cedd8))
*   run Postgres tests and clippy in CI (#1923) ([6c6c06ff](https://github.com/mozilla-services/syncstorage-rs/commit/6c6c06ffbe2ed71d5bfc562e1bfb86d663f4d9b3))
*   adjust dependabot pr limit to 1 (#1918) ([7103b3da](https://github.com/mozilla-services/syncstorage-rs/commit/7103b3dae0863ad24593963c996c11e3440de88c))
*   add healthcheck to prevent e2e tests starting too early ([ad0f6776](https://github.com/mozilla-services/syncstorage-rs/commit/ad0f6776a0654acd40ca92e7c6a585c62ba4fae5))
*   tokenserver-db-postgres -> tokenserver-postgres ([36657fd6](https://github.com/mozilla-services/syncstorage-rs/commit/36657fd62d70f6f6a7aa4582f045a9c881a28bec))
*   fix Cargo.lock ([9b008f8d](https://github.com/mozilla-services/syncstorage-rs/commit/9b008f8dc0aaab9e42965d346df28bd3f73e6e12))
*   async sync method suffix  (#1821) ([4a8ec959](https://github.com/mozilla-services/syncstorage-rs/commit/4a8ec9594a986775b10fe53ef071dfb69380d1e4))
*   allow overriding RUST_LOG (#1813) ([f2c7f944](https://github.com/mozilla-services/syncstorage-rs/commit/f2c7f9444085c9772da35797de9b6614635c7d88))
*   use environment value from syncserver settings for Sentry ([204c13e7](https://github.com/mozilla-services/syncstorage-rs/commit/204c13e75d5bd3cc8a471aee8f34a88964f673c5))
* **CI:**  fetch MySQL public key from MIT ([31e5e2ba](https://github.com/mozilla-services/syncstorage-rs/commit/31e5e2ba2f5ec44563d760869b5851683a043a9a))
* **ci:**
  *  use gh actions to build and deploy to GAR and ghcr (#1976) ([c46408b0](https://github.com/mozilla-services/syncstorage-rs/commit/c46408b0166a4e1e25d6f14ab87109cb2d2ac1c0))
  *  configure CircleCI for postgres builds ([30478687](https://github.com/mozilla-services/syncstorage-rs/commit/30478687d7ccadd3e4f00f6df143d02688a1a816))
* **deps:**
  *  bump quinn-proto from 0.11.13 to 0.11.14 (#2110) ([40a8618b](https://github.com/mozilla-services/syncstorage-rs/commit/40a8618bc74959cbb3686e47ebdcccde93b7bd3e))
  *  bump werkzeug in /tools/tokenserver/loadtests (#2077) ([35b0a9dd](https://github.com/mozilla-services/syncstorage-rs/commit/35b0a9dda871371135322590afef4aa86f0d8176))
  *  bump flask in /tools/tokenserver/loadtests (#2078) ([c556860a](https://github.com/mozilla-services/syncstorage-rs/commit/c556860a75759b77ab07518a8eecbab7e1e1be9b))
  *  bump cryptography in /tools/integration_tests (#2072) ([cc07bb51](https://github.com/mozilla-services/syncstorage-rs/commit/cc07bb51954f9a318df6d1a2c20448ca193da760))
  *  bump cryptography from 44.0.2 to 46.0.5 (#2054) ([ab6ec0a0](https://github.com/mozilla-services/syncstorage-rs/commit/ab6ec0a02bd6e5edc6e93b77f89111beee0bd167))
  *  bump cryptography in /tools/tokenserver/loadtests (#2055) ([d3880c56](https://github.com/mozilla-services/syncstorage-rs/commit/d3880c569cb114f9f8909e0acbfe76b00cc1d90e))
  *  bump bytes from 1.10.1 to 1.11.1 (#2039) ([854e6c5b](https://github.com/mozilla-services/syncstorage-rs/commit/854e6c5b7ef89e3e4d8c33db9b0486e45a5295e5))
  *  bump pyasn1 from 0.6.1 to 0.6.2 in /tools/spanner (#2008) ([cccdac90](https://github.com/mozilla-services/syncstorage-rs/commit/cccdac909d843c17c5e5266680f21a02a886f27b))
  *  bump urllib3 from 2.6.0 to 2.6.3 in /tools/spanner (#1994) ([ea0055df](https://github.com/mozilla-services/syncstorage-rs/commit/ea0055df753022609af46f3a3d03c11d941d7310))
  *  bump authlib in /tools/tokenserver/loadtests (#1992) ([230b4a38](https://github.com/mozilla-services/syncstorage-rs/commit/230b4a386262bd28e143e5e799883d3fa34b8e40))
  *  bump urllib3 in /tools/integration_tests (#1985) ([fbbd87e0](https://github.com/mozilla-services/syncstorage-rs/commit/fbbd87e0b5dccf0b033610217a1c5d5e0cf1cd50))
  *  bump urllib3 from 2.6.0 to 2.6.3 in /tools/tokenserver (#1983) ([c13f7d7c](https://github.com/mozilla-services/syncstorage-rs/commit/c13f7d7c569e7b7b23b5e31afc4651fcab7945fd))
  *  bump urllib3 from 2.5.0 to 2.6.3 (#1982) ([1a4a0e48](https://github.com/mozilla-services/syncstorage-rs/commit/1a4a0e486cdddaa4e16a5f9d79a4e53e543c3baa))
  *  bump urllib3 in /tools/integration_tests (#1978) ([9612bc95](https://github.com/mozilla-services/syncstorage-rs/commit/9612bc9504620b06043d2c4c12ec075f748d35b7))
  *  bump urllib3 from 2.5.0 to 2.6.0 in /tools/tokenserver ([4ab5dba8](https://github.com/mozilla-services/syncstorage-rs/commit/4ab5dba88dbde58842bb43b665a68c615051ab23))
  *  bump urllib3 from 2.5.0 to 2.6.0 in /tools/spanner ([78ee28e7](https://github.com/mozilla-services/syncstorage-rs/commit/78ee28e70a4ce963db9c4ca5b6326ebd977075e0))
  *  bump authlib in /tools/tokenserver/loadtests ([207a010e](https://github.com/mozilla-services/syncstorage-rs/commit/207a010ed0069975dc1411a197c7fb1b054b37d6))
  *  bump authlib in /tools/tokenserver/loadtests (#1816) ([f60189d9](https://github.com/mozilla-services/syncstorage-rs/commit/f60189d9a0e8a50816373c9750ee4fc14697f685))
* **deps-dev:**
  *  bump werkzeug in /tools/tokenserver/loadtests (#1990) ([0e08ba60](https://github.com/mozilla-services/syncstorage-rs/commit/0e08ba60f59ba84cf99d5a7ad2506d9d8d790211))
  *  bump urllib3 in /tools/tokenserver/loadtests (#1984) ([cba780e5](https://github.com/mozilla-services/syncstorage-rs/commit/cba780e5df3a7453e88ca90098a0e2eca9de209e))
  *  bump urllib3 in /tools/tokenserver/loadtests ([77836c6c](https://github.com/mozilla-services/syncstorage-rs/commit/77836c6ce10f7f961e4908711071617b63b95b47))
  *  bump werkzeug in /tools/tokenserver/loadtests ([740b5cc9](https://github.com/mozilla-services/syncstorage-rs/commit/740b5cc9aa51152864b0a6316babeefc8e50c07c))
  *  bump brotli in /tools/tokenserver/loadtests ([7b907bdf](https://github.com/mozilla-services/syncstorage-rs/commit/7b907bdfd7e4ce685c6e75c3e650565788fbf5fc))
* **syncserver:**  break up extractors.rs ([34041507](https://github.com/mozilla-services/syncstorage-rs/commit/34041507925fce35f85f6c657d23e629f56763fc))

#### Doc

*   general bootstrapping instructions postgres (#2088) ([03627ed3](https://github.com/mozilla-services/syncstorage-rs/commit/03627ed3d8e3c5b6ddf057edbb3ded36f89bbdd9))
*   content from archived repos (#2075) ([a4758569](https://github.com/mozilla-services/syncstorage-rs/commit/a4758569f4c5bf5b028f4a9efe86dbe0e984fe53))
*   load test runs for locust and molotov (#2084) ([30258c82](https://github.com/mozilla-services/syncstorage-rs/commit/30258c82f34b919522bd627debd085aa0513b8b5))
*   open api docs utoipa (#2023) ([62bd7d2c](https://github.com/mozilla-services/syncstorage-rs/commit/62bd7d2c6a8bc6117950bc437ebda753036bfc0a))
*   update docs for prepare-spanner (#2035) ([9d0a6391](https://github.com/mozilla-services/syncstorage-rs/commit/9d0a63917ded147f2b0cdd727b08613f8954b26b))
*   update settings list (#2030) ([b9ea1191](https://github.com/mozilla-services/syncstorage-rs/commit/b9ea11918beefc4506af4d32401234313da186e7))
*   add one-shot docker compose yaml to how-to (#2025) ([646e516e](https://github.com/mozilla-services/syncstorage-rs/commit/646e516eeca46fde3ad13bf064ad2d2672df9483))
*   add how-to on deploying with docker compose (#2019) ([11659d98](https://github.com/mozilla-services/syncstorage-rs/commit/11659d98f9c69948a0aab353437ce2036c388711))
*   sync api docs github (#1986) ([77868a38](https://github.com/mozilla-services/syncstorage-rs/commit/77868a3890ce63e774c82ec0ef4ac11fd371b4ff))
*   architecture and system diagrams  (#1973) ([a108a0da](https://github.com/mozilla-services/syncstorage-rs/commit/a108a0da8e040d9dc3522f731c25bef1304b37db))
*   build deploy docs GitHub pages (#1965) ([8b72a3b8](https://github.com/mozilla-services/syncstorage-rs/commit/8b72a3b8dfea25750b1629d3027a6fda63064233))
*   contributing guidelines fix  (#1868) ([9170b97f](https://github.com/mozilla-services/syncstorage-rs/commit/9170b97f276df8d1c81247660fb1e75f7198079a))
*   update readme for project setup ([494b1617](https://github.com/mozilla-services/syncstorage-rs/commit/494b16179555fe404f8ec1d16721a941b02ebc45))



<a name="0.21.1"></a>
## 0.21.1 (2025-09-23)


#### Bug Fixes

*   switch check to SELECT 1 to fix it on diesel-async (#1818) ([f9d142fb](https://github.com/mozilla-services/syncstorage-rs/commit/f9d142fbee7b8998e58f1d783d800e03466ba728))



<a name="0.21.0"></a>
## 0.21.0 (2025-09-11)


#### Refactor

*   move tokenserver's Db to async-trait (#1799) ([9d799a45](https://github.com/mozilla-services/syncstorage-rs/commit/9d799a45d41d1f09589cd01ac4df38decb7a9548))
*   apply &mut self to syncstorage-mysql ([38b4db40](https://github.com/mozilla-services/syncstorage-rs/commit/38b4db401fdcb405cb7f56e54e85aca47877265d))
*   switch syncstorage Db methods to &mut self ([38cb38fb](https://github.com/mozilla-services/syncstorage-rs/commit/38cb38fbd91861db8ac9ce28d9149bb5db530fca))
*   switch tokenserver Db methods to &mut self ([614e3902](https://github.com/mozilla-services/syncstorage-rs/commit/614e3902d00749b478b84fbdfa7b7247ab1b56cd))

#### Bug Fixes

*   kill unnecessary transactions (savepoints) (#1782) ([9381bc68](https://github.com/mozilla-services/syncstorage-rs/commit/9381bc68b8c9a1b73806790f1803a0ec7f3e410b))
*   Revert "fix: mysql: Call set_timestamp in lock_for_write_sync." ([dfe21646](https://github.com/mozilla-services/syncstorage-rs/commit/dfe216466da41581cae084e19db2b43fcf8fca77))
*   mysql: Replace the user_id%10 in batch_id with a global counter. ([2a1268ed](https://github.com/mozilla-services/syncstorage-rs/commit/2a1268ed0440de4c5709b150b37c440369bf2a2e))
*   mysql: Call set_timestamp in lock_for_write_sync. ([d4511cc7](https://github.com/mozilla-services/syncstorage-rs/commit/d4511cc746d8c2e991c8faf55ef608e8ff7895bc))
*   Fix usage of Mockito for >0.30.0. ([3e37e42d](https://github.com/mozilla-services/syncstorage-rs/commit/3e37e42d92cda1cb9e072a279145dc9a369cf22c))
*   Fix misspelled cfg(test) in tokenserver-auth. ([82dd4235](https://github.com/mozilla-services/syncstorage-rs/commit/82dd4235c665d9c41800b137af920557d7321ef4))

#### Test

*   add max total records e2e test (#1796) ([6100529d](https://github.com/mozilla-services/syncstorage-rs/commit/6100529da70bb1af2de6f329c8b63224113da2a3))

#### Chore

*   more poetry usage (#1798) ([ffade2f8](https://github.com/mozilla-services/syncstorage-rs/commit/ffade2f86909cb6cf29264d453be839afdf7065f))
*   bump tracing-subscriber per RUSTSEC-2025-0055 ([d89a05e7](https://github.com/mozilla-services/syncstorage-rs/commit/d89a05e7451fe797ad79ba00ecc1960ba4c94c13))
*   Bump validator to 0.19.0. ([c924fae7](https://github.com/mozilla-services/syncstorage-rs/commit/c924fae77a98f995db64bb5e5711f313553de1e5))
*   Update Cargo.lock. ([dc4e8015](https://github.com/mozilla-services/syncstorage-rs/commit/dc4e80151b654e58bf881f940b82f3171c06366a))
*   Upgrade Diesel to 2.x. ([b9507e36](https://github.com/mozilla-services/syncstorage-rs/commit/b9507e36c397d7114691498f3bb67c78fa74ac1b))

#### Features

*   tokenserver postgres schema (#1786) ([1dd7dab5](https://github.com/mozilla-services/syncstorage-rs/commit/1dd7dab55ec3aaf2658019dde4b1ac8e70c668a4))
*   switch tokenserver to diesel-async (#1790) ([f2698a42](https://github.com/mozilla-services/syncstorage-rs/commit/f2698a4251cd719413b2071379f6866b50d4981a))
*   kill Db's impl of Clone (#1789) ([b89b06d2](https://github.com/mozilla-services/syncstorage-rs/commit/b89b06d24f4b276c8818e1b062e9431944c5f59f))
*   workaround batch_id conflicts w/ a simple retry ([e71980c2](https://github.com/mozilla-services/syncstorage-rs/commit/e71980c24281029854165bb5a74cf736aa41b5e4))



<a name="0.20.1"></a>
## 0.20.1 (2025-08-29)


#### Features

*   bump max payload to 2.5 (#1772) ([7c4b7c1b](https://github.com/mozilla-services/syncstorage-rs/commit/7c4b7c1b8f9ec89c7ac5612393f92623cbc7e797))

#### Chore

*   update sync python version  (#1774) ([19b6176d](https://github.com/mozilla-services/syncstorage-rs/commit/19b6176d5b9d7848b57954a489d494394c175798))



<a name="0.20.0"></a>
## 0.20.0 (2025-08-14)


#### Test

*   resolve test deprecations (#1732) ([8055e742](https://github.com/mozilla-services/syncstorage-rs/commit/8055e7429d376a33a9434f05f33ce0d7dabbd825))
*   collect and report spanner tests (#1743) ([3e130960](https://github.com/mozilla-services/syncstorage-rs/commit/3e130960441334c3c17042833309a679e2160026))

#### Chore

*   update syncstorage rust (#1749) ([f7197fef](https://github.com/mozilla-services/syncstorage-rs/commit/f7197fefb8d23ec9634dc1c70e473ca08a70fae5))

#### Features

*   emit sentry events for INVALID_ARGUMENT (#1748) ([57af35c5](https://github.com/mozilla-services/syncstorage-rs/commit/57af35c591b5b12dde6ed3405ae2932dc2e3ff33))
*   ruff for python lint and format (#1742) ([8995db62](https://github.com/mozilla-services/syncstorage-rs/commit/8995db6268f5745cd671a43a14903f0ee6631ce5))



<a name="0.19.1"></a>
## 0.19.1 (2025-08-07)


#### Features

*   quiet pool timeout events (emit as metrics) (#1740) ([3e20b054](https://github.com/mozilla-services/syncstorage-rs/commit/3e20b054cf2dd274fe551fe40bfb4b946812a2ea))



<a name="0.19.0"></a>
## 0.19.0 (2025-08-05)


#### Refactor

*   python imports (#1730) ([77254b4a](https://github.com/mozilla-services/syncstorage-rs/commit/77254b4a6eb5f9806e103dcd67ad667412078fb4))
*   convert db extract calls to async (#1715) ([4ddf5b41](https://github.com/mozilla-services/syncstorage-rs/commit/4ddf5b416962b29e4257aac964922c16701f74a9))
*   sync sentry cleanup grpc errors (#1716) ([b0c8ac50](https://github.com/mozilla-services/syncstorage-rs/commit/b0c8ac50a0ec4ebb3e19ecb9a37eb7b9501da507))
*   remove user migration utils (#1710) ([f01c21fe](https://github.com/mozilla-services/syncstorage-rs/commit/f01c21fef456e43dc5b73cb4882ea51f95f42ab6))
*   move stream code into its own module ([a51c0144](https://github.com/mozilla-services/syncstorage-rs/commit/a51c01445eda08721418c9cc5eda389f6b7df347))
*   remove purge_ttl.rs  (#1702) ([31c3b866](https://github.com/mozilla-services/syncstorage-rs/commit/31c3b866a4f69fe313f0e1979ac4120bc7d764e8))
*   quiet dbg calls (#1701) ([0e3f7d16](https://github.com/mozilla-services/syncstorage-rs/commit/0e3f7d162c9cea4a68524e1b43151aea51be612f))

#### Bug Fixes

*   make pyo3 usage optional per the feature flag (#1731) ([2fb6b84a](https://github.com/mozilla-services/syncstorage-rs/commit/2fb6b84ad46755eeb6445071445430c7fc05fde8))

#### Breaking Changes

*   require minimum mysql 8 (#1717) ([69005091](https://github.com/mozilla-services/syncstorage-rs/commit/69005091a5d9e3adca246d95ebee97b44d241dce), breaks [#](https://github.com/mozilla-services/syncstorage-rs/issues/))

#### Chore

*   require minimum mysql 8 (#1717) ([69005091](https://github.com/mozilla-services/syncstorage-rs/commit/69005091a5d9e3adca246d95ebee97b44d241dce), breaks [#](https://github.com/mozilla-services/syncstorage-rs/issues/))
*   strip actix-web features (#1713) ([dd546f02](https://github.com/mozilla-services/syncstorage-rs/commit/dd546f028e771845c174b66e61f02714a3d9a612))
*   remove extraneous dependencies (#1700) ([0768d497](https://github.com/mozilla-services/syncstorage-rs/commit/0768d4975de555b2c7af64dfef1ba40e5e2f99df))

#### Features

*   use poetry for dependency management (#1706) ([f8715d4e](https://github.com/mozilla-services/syncstorage-rs/commit/f8715d4e916e5f3ef5431cb40ecdebd71b21fa7d))
*   spanner scripts parse gcp project (#1714) ([d716ac5d](https://github.com/mozilla-services/syncstorage-rs/commit/d716ac5d105cb9eb8a603f2750bd3c3f0361837a))

#### Test

*   add spanner db tests to ci (#1711) ([f407eb21](https://github.com/mozilla-services/syncstorage-rs/commit/f407eb21d1f2fe987e05ead9d55dc33c0d225b01))
*   make StreamedResultSet's stream generic ([59df9f64](https://github.com/mozilla-services/syncstorage-rs/commit/59df9f64276b5e1f7fe2ab12929c0425181d87c2))



<a name="0.18.3"></a>
## 0.18.3 (2025-05-14)


#### Chore

*   bump to latest rust ([0148e04d](https://github.com/mozilla-services/syncstorage-rs/commit/0148e04dd2881869ffe52b6ebb93be6929f31a25))
*   update python cryptography (#1690) ([e93bb882](https://github.com/mozilla-services/syncstorage-rs/commit/e93bb8821ccdf94e34c184f51ad86f0388333f3d))
*   added build-and-push to GAR (#1654) ([cb37e2aa](https://github.com/mozilla-services/syncstorage-rs/commit/cb37e2aa4134d5e8e0c11178e267d3e7565da05d))
*   upload test artifacts to gcs ([aeedcf1e](https://github.com/mozilla-services/syncstorage-rs/commit/aeedcf1e19e622b4f0d0e9c813ba4da3c712f125))
*   switch back to libmariadb-dev (#1665) ([e0093a88](https://github.com/mozilla-services/syncstorage-rs/commit/e0093a88bfc059a891c1a5d3f74cef068b720861))
*   migrate tokenserver tests to pytest with junit output ([15840c5e](https://github.com/mozilla-services/syncstorage-rs/commit/15840c5ecfd1e6fbcd239bed0f50cf3537631775))
*   migrate unit tests to nextest and llvm-cov ([8c56cae8](https://github.com/mozilla-services/syncstorage-rs/commit/8c56cae8905325345972a4abe99c12c1fc1b012c))

#### Features

*   build docker w/ Oracle's libmysqlclient (#1695) ([569e5100](https://github.com/mozilla-services/syncstorage-rs/commit/569e5100839245cd5869bb12b655b7fe571fbbcf))
*   emit oauth verification timeouts as metrics (not sentry) (#1694) ([624eced1](https://github.com/mozilla-services/syncstorage-rs/commit/624eced1e9cad6492a38397c9440b558d263cca0))

#### Bug Fixes

*   re-enable tokensever e2e tests ([d0336c88](https://github.com/mozilla-services/syncstorage-rs/commit/d0336c8869e52a48e49fed989b5ac9573a3b1e55))
*   avoid underflow of the queued_tasks metric ([10daab06](https://github.com/mozilla-services/syncstorage-rs/commit/10daab06cf35cf5696aa6ed6b790d8115bfeb432))
*   Revert "fix: avoid underflow of the queued_tasks metric (#1628)" ([31dda136](https://github.com/mozilla-services/syncstorage-rs/commit/31dda136809879b8e7f91f095bc378bb41b9f304))
*   resolve pyo3 vuln deprecations (#1682) ([0675930a](https://github.com/mozilla-services/syncstorage-rs/commit/0675930a155d27bbf2eca2c0abf81d262a9cfb28))
* **infra:**  configure gcp utils before upload (#1698) ([5dcfefe2](https://github.com/mozilla-services/syncstorage-rs/commit/5dcfefe2b6a8946f02c7bfac2fd641b0a6a3356b))

#### Test

* **e2e:**  run integration and e2e tests with pytest (#1697) ([6f15ad54](https://github.com/mozilla-services/syncstorage-rs/commit/6f15ad546d3c5234986db09fec485fb911624e5f))

#### Doc

*   add tokenserver documentation to sync (#1681) ([dadbcea3](https://github.com/mozilla-services/syncstorage-rs/commit/dadbcea3f7428ad7f0a5ae6f0c2ad966c331660a))
*   update purge script's status now that it's running (#1679) ([6f0c7b28](https://github.com/mozilla-services/syncstorage-rs/commit/6f0c7b28db3f8a2701c4af4dfe7a2d691fc079ef))
*   document pruning scripts (#1645) ([7c9bc008](https://github.com/mozilla-services/syncstorage-rs/commit/7c9bc0089dd73a9ecaba8b33e26634b2a69b5ff0))

#### Refactor

*   kill tokenserver's TokenType now that it's solely oauth ([a26ff490](https://github.com/mozilla-services/syncstorage-rs/commit/a26ff490b8086ce3c12b837ca00cc757caa54169))
*   simplify metric_label to return a &str ([0ca435fb](https://github.com/mozilla-services/syncstorage-rs/commit/0ca435fb1a05f073d1e78ed420d953a00c8d0d53))



<a name="0.18.2"></a>
## 0.18.2 (2024-12-05)


#### Chore

*   bump to latest sentry (#1639) ([bc79ccb9](https://github.com/mozilla-services/syncstorage-rs/commit/bc79ccb97243f946c1abb436f07a1be8b63f6ba6))



<a name="0.18.1"></a>
## 0.18.1 (2024-11-27)


#### Features

*    Enable Glean probe-scraper task (#1636) ([8363f82d](https://github.com/mozilla-services/syncstorage-rs/commit/8363f82d4197923e8ee1062de849d2c61e467db4))



<a name="0.18.0"></a>
## 0.18.0 (2024-11-26)


#### Doc

*   sync DAU server side metrics adr (#1608) ([7e211542](https://github.com/mozilla-services/syncstorage-rs/commit/7e21154203411e98200e7af60e2e7199050e9fb7))

#### Features

*   glean metrics logic (#1626) ([9e9869ee](https://github.com/mozilla-services/syncstorage-rs/commit/9e9869ee0605d0610d6c94bf6185eb1eabd6b6a2))



<a name="0.17.15"></a>
## 0.17.15 (2024-11-21)


#### Bug Fixes

*   upgrade to latest deadpool (#1631) ([9a97b6ce](https://github.com/mozilla-services/syncstorage-rs/commit/9a97b6ce1ae8295ea45ba017d8b0ef81ec1cf694))



<a name="0.17.14"></a>
## 0.17.14 (2024-11-19)


#### Bug Fixes

*   don't add extra prefixes to middleware emitted metrics (#1630) ([9b033edc](https://github.com/mozilla-services/syncstorage-rs/commit/9b033edcb0a6479bdb7fe02e50602f85bf41cf8f))
*   avoid underflow of the queued_tasks metric (#1628) ([3ed6d607](https://github.com/mozilla-services/syncstorage-rs/commit/3ed6d6077cf987f31d35e3ff772cfbb5f81f5b73))

#### Features

*   add metric values to get_collections (#1616) ([98ccc954](https://github.com/mozilla-services/syncstorage-rs/commit/98ccc95482e79ed038abcdb87f6ef5cacaee0bf2))



<a name="0.17.13"></a>
## 0.17.13 (2024-10-30)


#### Features

*   namespace the db error labels (#1625) ([bab5e1fe](https://github.com/mozilla-services/syncstorage-rs/commit/bab5e1fe51ef13fb36810cde93347d61372ae57c))



<a name="0.17.12"></a>
## 0.17.12 (2024-10-29)


#### Bug Fixes

*   upgrade sentry w/ a fix for the blocking curl Transport (#1621) ([b8641a6c](https://github.com/mozilla-services/syncstorage-rs/commit/b8641a6cabd8ad043956fa8cb478dd6db25ca58a))

#### Features

*   glean metrics data review (#1609) ([c8ec7cab](https://github.com/mozilla-services/syncstorage-rs/commit/c8ec7cab68d132a8d2a3230c49627db5da62db63))
*   add hashed_device_id to HawkIdentifier (#1615) ([cc6dd137](https://github.com/mozilla-services/syncstorage-rs/commit/cc6dd13749a61793a715ab4775074090588c75a1))



<a name="0.17.11"></a>
## 0.17.11 (2024-10-22)


#### Features

*   Revert "feat: Revert "fix: revert the python3.10 match statement (for now) (#1592)"" ([1b13123a](https://github.com/mozilla-services/syncstorage-rs/commit/1b13123a2b9a61d53f03c7f89672c6fbb7568f2d))
*   revert "feat: Revert "chore: revert back to bullseye (for now) (#1589)"" ([e170518c](https://github.com/mozilla-services/syncstorage-rs/commit/e170518c0f5696ed51478fecafc1a59eca176053))
*   add hashed_fxa_uid to HawkPayload  (#1613) ([715cf950](https://github.com/mozilla-services/syncstorage-rs/commit/715cf950ba22d25d85264ecb6360305b29ec70eb))
*   user agent parsing (#1607) ([7f2ef062](https://github.com/mozilla-services/syncstorage-rs/commit/7f2ef062fc71e749a00f4d960e70380c4fe44ea1))



<a name="0.17.10"></a>
## 0.17.10 (2024-10-19)


#### Features

*   wire MysqlError's ReportableError impl into TokenserverError (#1611) ([c535e5ae](https://github.com/mozilla-services/syncstorage-rs/commit/c535e5ae52d03ee1c2df287c3bbed6c2321f377b))
*   create DAU glean schema and configs (#1606) ([d2313310](https://github.com/mozilla-services/syncstorage-rs/commit/d23133101f5367e2070a0cc5b711e756f36f5b72))
*   track the pool's queued vs actually active tasks (#1605) ([1f0e28d7](https://github.com/mozilla-services/syncstorage-rs/commit/1f0e28d7af9c6f9aea38073a099699897464ceac))



<a name="0.17.9"></a>
## 0.17.9 (2024-09-26)


#### Bug Fixes

*   ensure the pool counter's always decremented via scopeguard (#1604) ([4259183a](https://github.com/mozilla-services/syncstorage-rs/commit/4259183ae4ef71efb7cd77db9b9d0e1637ca8dc2))



<a name="0.17.8"></a>
## 0.17.8 (2024-09-24)


#### Chore

* **deps:**  bump cryptography in /tools/integration_tests (#1594) ([be23e391](https://github.com/mozilla-services/syncstorage-rs/commit/be23e39135d58ecaee917c49bf14aa52a406ccea))

#### Bug Fixes

*   correctly read the SYNC_STATSD_HOST/PORT settings (#1601) ([3675c938](https://github.com/mozilla-services/syncstorage-rs/commit/3675c9387b8418a1a67dd13d95b338e12ca5dae3))



<a name="0.17.7"></a>
## 0.17.7 (2024-09-19)


#### Bug Fixes

*   correct TokenserverError's sentry "type"/"value" fields ([bbd5abac](https://github.com/mozilla-services/syncstorage-rs/commit/bbd5abac8e060d0083aaec3c3d8f88c374d44828))

#### Refactor

*   move sentry middlware and Taggable to syncserver-common ([5d9d203c](https://github.com/mozilla-services/syncstorage-rs/commit/5d9d203c62aa1f4df7c627c37eb0bc6c47ddae0b))

#### Features

*   Revert "fix: revert the python3.10 match statement (for now) (#1592)" ([f3bdda91](https://github.com/mozilla-services/syncstorage-rs/commit/f3bdda91660a6777b715b59253234c4d7ba4a520))
*   Revert "chore: revert back to bullseye (for now) (#1589)" ([bbdfb193](https://github.com/mozilla-services/syncstorage-rs/commit/bbdfb1933dc557ae23fabcb87eb5a22e4478a069))



<a name="0.17.6"></a>
## 0.17.6 (2024-09-17)


#### Features

*   pickup the syncserver metrics settings (#1598) ([b52e44ab](https://github.com/mozilla-services/syncstorage-rs/commit/b52e44ab52796b30bf94f39d7db54ae3981c6437))



<a name="0.17.5"></a>
## 0.17.5 (2024-09-12)


#### Bug Fixes

*   downcast to tokenserver's actual error type (#1596) ([2b8b1f5d](https://github.com/mozilla-services/syncstorage-rs/commit/2b8b1f5dde7fbb5717ad2d7c292f9dbf69b0d271))



<a name="0.17.4"></a>
## 0.17.4 (2024-09-06)


#### Features

*   debug "Invalid OAuth token" (verifier returns None) error cases (#1595) ([1443b31e](https://github.com/mozilla-services/syncstorage-rs/commit/1443b31e5af1f10f8a52bf1bb91dc817ce0b75f2))



<a name="0.17.3"></a>
## 0.17.3 (2024-08-30)


#### Bug Fixes

*   revert the python3.10 match statement (for now) (#1592) ([dc0d571c](https://github.com/mozilla-services/syncstorage-rs/commit/dc0d571c055741297a77dd47c70b7ef55b552530))



<a name="0.17.2"></a>
## 0.17.2 (2024-08-07)


#### Chore

*   revert back to bullseye (for now) (#1589) ([4a503f8c](https://github.com/mozilla-services/syncstorage-rs/commit/4a503f8c36fe070e11df43a8ce0b3c71358e983c))

#### Doc

*   add missing changelog for dep updates ([68db54b5](https://github.com/mozilla-services/syncstorage-rs/commit/68db54b5ce226d96da449d501a08d15392a35122))



<a name="0.17.1"></a>
## 0.17.1 (2024-07-11)


#### Chore

*   Updates for Jun-2024 (#1576) ([1713962c](https://github.com/mozilla-services/syncstorage-rs/commit/1713962c6a48ca5d2a0efd4fac739482649b650c))

#### Doc

*   clarify the handling of existing expired bsos in writes (#1581) ([250ac943](https://github.com/mozilla-services/syncstorage-rs/commit/250ac94353d0fdd0c387bb69f5ab90aa28a4689d), closes [#619](https://github.com/mozilla-services/syncstorage-rs/issues/619))

#### Bug Fixes

*   don't hide TokenserverPool initialization errors on startup (#1584) ([1edce041](https://github.com/mozilla-services/syncstorage-rs/commit/1edce04154d354e78994621a0b88ddf42fb7ff66))



<a name="0.17.0"></a>
## 0.17.0 (2024-06-15)


#### Chore

*   bump crytography/pyramid to quiet a number of security alerts (#1574) ([6c9b771b](https://github.com/mozilla-services/syncstorage-rs/commit/6c9b771bc576207d642f91bf69c4fce21a98e4c3))

#### Bug Fixes

*   Revert the venv configuration for python (#1571) ([0f86587e](https://github.com/mozilla-services/syncstorage-rs/commit/0f86587edd5cf35263558e7e72e808e11f2612fd))

#### Features

*   Remove support for BrowserID (#1531) ([dbbdd1df](https://github.com/mozilla-services/syncstorage-rs/commit/dbbdd1dfc3a130be46d4586133daa36c67378e7a))



<a name="0.16.0"></a>
## 0.16.0 (2024-06-11)


#### Chore

*   Update to debian bookworm / Python 3.12 (#1567) ([8f9e1c27](https://github.com/mozilla-services/syncstorage-rs/commit/8f9e1c27cf8dc9e6bc176a98cc049e9735330e43))



<a name="0.15.9"></a>
## 0.15.9 (2024-05-31)


#### Features

*   Add timeouts for tokenserver database calls. (#1561) ([2584b977](https://github.com/mozilla-services/syncstorage-rs/commit/2584b977b8a315a571066c0a417e76401b14bdfd))
*   Add metrics, gcp logging to tokenserver scripts (#1555) ([6537783a](https://github.com/mozilla-services/syncstorage-rs/commit/6537783a9c3781802fd16478867e912868f7f8d7))
*   Add normalized ReportableError to errors (#1559) ([77181308](https://github.com/mozilla-services/syncstorage-rs/commit/771813087c8eccc448530cea2d323f8de8ee81a3))

#### Bug Fixes

*   nix-shell: update `pkgconfig` -> `pkg-config` build input (#1562) ([a55e3738](https://github.com/mozilla-services/syncstorage-rs/commit/a55e373823ac2c54280a9633f67143ff29ec828b))
*   Allow threadpool size to be set. (#1560) ([ab7b4221](https://github.com/mozilla-services/syncstorage-rs/commit/ab7b4221fd664e23604a77041746f6f12a0a7d7e))

#### Doc

*   Remove commented code, unneeded TODO, unneeded collision tracking (#1563) ([5cdfd034](https://github.com/mozilla-services/syncstorage-rs/commit/5cdfd03498055865fc27a53e263303355ac5fdb0))



<a name="0.15.8"></a>
## 0.15.8 (2024-05-08)


#### Features

*   Ignore non-spanner nodes for scripts (#1557) ([581c2507](https://github.com/mozilla-services/syncstorage-rs/commit/581c250739f0f51f392dc5dc5984924395545791))



<a name="0.15.7"></a>
## 0.15.7 (2024-05-02)


#### Features

*   optionally force the spanner node via get_best_node (#1553) ([4a145dd1](https://github.com/mozilla-services/syncstorage-rs/commit/4a145dd18bc13345179dbaedf6a0ae2d31ad4281))



<a name="0.15.6"></a>
## 0.15.6 (2024-04-30)


#### Bug Fixes

*   validate val names (#1550) ([5dc53f22](https://github.com/mozilla-services/syncstorage-rs/commit/5dc53f2282d1d97c3b5baf730bb4b8165f06d8a1))



<a name="0.15.5"></a>
## 0.15.5 (2024-04-30)


#### Features

*   Allow uid range for purge function (SYNC-4246) (#1547) ([cc160822](https://github.com/mozilla-services/syncstorage-rs/commit/cc160822419cd56646d15d425812cf36a19d89a2), closes [#1548](https://github.com/mozilla-services/syncstorage-rs/issues/1548))



<a name="0.15.4"></a>
## 0.15.4 (2024-04-25)


#### Bug Fixes

*   take keys_changed_at into account w/ migrated records' special case (#1545) ([f68fb607](https://github.com/mozilla-services/syncstorage-rs/commit/f68fb607fe0284f74c77faa4eb1de14ed95e3d3e))

#### Chore

*   fix changelog version anchor ([8098d839](https://github.com/mozilla-services/syncstorage-rs/commit/8098d839b6987bfa0731f876162672bb21e8fded))



<a name="0.15.3"></a>
## 0.15.3 (2024-04-24)


#### Features

*   special case purging of users previously migrated to Spanner (#1543) ([13e53eba](https://github.com/mozilla-services/syncstorage-rs/commit/13e53eba13ca21f8bd41ddd86d52375f4af38a71))



<a name="0.15.2"></a>
## 0.15.2 (2024-04-16)


#### Bug Fixes

*   Add try/except handler for force (#1535) ([b777fa0d](https://github.com/mozilla-services/syncstorage-rs/commit/b777fa0d967472ca34b023c606cfc5ef5309bf73))
*   add line break to do not display backticks (#1529) ([143e93b6](https://github.com/mozilla-services/syncstorage-rs/commit/143e93b66f27e0d03509d17db8da53f9397fe73e))

#### Chore

*   bump mio per RUSTSEC-2024-0019 (#1530) ([b4306d93](https://github.com/mozilla-services/syncstorage-rs/commit/b4306d9379930ab6602a4efdb1278e4eb302b567))



<a name="0.15.1"></a>
## 0.15.1 (2024-02-29)


#### Bug Fixes

*   don't emit a content-type header for 304s (#1526) ([8faf7280](https://github.com/mozilla-services/syncstorage-rs/commit/8faf7280de843b5d398aeb997c99aebfdc5d9a8c))

#### Doc

*   Remove reference to legacy vendored library (#1522) ([3edd4206](https://github.com/mozilla-services/syncstorage-rs/commit/3edd420621520e073dff0828fd7b30579a4c9349))



<a name="0.15.0"></a>
## 0.15.0 (2024-02-27)


#### Features

*   Puts pyo3 behind feature flag and derives tokens directly in Rust (#1513) ([1b116846](https://github.com/mozilla-services/syncstorage-rs/commit/1b11684648f2b6e632b1ef286c62008278cb4c08))
*   Upgrading to Actix 4.x (#1514) ([97985586](https://github.com/mozilla-services/syncstorage-rs/commit/97985586b464976923bede595c40a05def2c0a64))

#### Bug Fixes

*   Copy modified purge script from old tokenserver  repo (#1512) ([06ecb78e](https://github.com/mozilla-services/syncstorage-rs/commit/06ecb78e2414c9fd7385709d19987ac8a1d1fa3d))

#### Chore

*   Cleans up TLS dependencies (#1519) ([ac3b479a](https://github.com/mozilla-services/syncstorage-rs/commit/ac3b479a58275d16e3529a17ef68521564b8b571))



<a name="0.14.4"></a>
## 0.14.4 (2023-12-11)


#### Bug Fixes

*   Use google specified UA for x-goog-api-client (#1506) ([9916b3bd](https://github.com/mozilla-services/syncstorage-rs/commit/9916b3bdb0506e9805f505007222f189f1c4dc54))



<a name="0.14.3"></a>
## 0.14.3 (2023-11-30)


#### Bug Fixes

*   restore emitting backend specific db errors to sentry (#1500) ([18f4d594](https://github.com/mozilla-services/syncstorage-rs/commit/18f4d594905e9dd4affc557a5da39cd19b6882f7))



<a name="0.14.2"></a>
## 0.14.2 (2023-11-21)


#### Doc

*   remove outdated firefox-ios warning (#1497) ([59283b59](https://github.com/mozilla-services/syncstorage-rs/commit/59283b5977a5d997c1292eb5392f6ad252855c0d))

#### Bug Fixes

*   disable sentry's debug-images feature (#1499) ([8d9185e4](https://github.com/mozilla-services/syncstorage-rs/commit/8d9185e4a012b1113f0a89d3d2852b55c7449114))



<a name="0.14.1"></a>
## 0.14.1 (2023-10-18)


#### Features

*   add dynamic routing headers to all Spanner ops (#1491) ([af416fc2](https://github.com/mozilla-services/syncstorage-rs/commit/af416fc29f51ef48115ff876b4cd99e274631d0a))

#### Chore

*   missed fixes from rollup (#1492) ([68d32670](https://github.com/mozilla-services/syncstorage-rs/commit/68d326701505a7c3ae04d59953eb099cf8add4d2))
*   tag 0.14.0 (#1485) ([c563ce5b](https://github.com/mozilla-services/syncstorage-rs/commit/c563ce5ba9006d4b12324a0912e765b2c562c01c))

#### Bug Fixes

*   switch more test flags to cfg(debug_assertions) (#1488) ([fb701288](https://github.com/mozilla-services/syncstorage-rs/commit/fb701288244daeee18a3ec26c986b6e6a98bb4f8))



<a name="0.14.0"></a>
## 0.14.0 (2023-09-26)


#### Refactor

*   quiet latest clippy warnings ([dc98e95f](https://github.com/mozilla-services/syncstorage-rs/commit/dc98e95ff3a59c267df7807ce9320d8b5a348b63))
*   add tokenserver-auth crate (#1413) ([ab5df9ba](https://github.com/mozilla-services/syncstorage-rs/commit/ab5df9ba79651fd2ed6a2374f39b6f0e060dac49), closes [#1278](https://github.com/mozilla-services/syncstorage-rs/issues/1278))
*   add database crates (#1407) ([b5b7e57f](https://github.com/mozilla-services/syncstorage-rs/commit/b5b7e57f935703f2c4207ad88eaa310c343fdb94), closes [#1277](https://github.com/mozilla-services/syncstorage-rs/issues/1277))
*   convert middleware to `wrap_fn` paradigm (#1374) ([973e90fa](https://github.com/mozilla-services/syncstorage-rs/commit/973e90fae88f104b6fb66d4f49a1c76472816e4a), closes [#714](https://github.com/mozilla-services/syncstorage-rs/issues/714))

#### Features

*   convert dependencies to use `workspace`s ([1f9323b7](https://github.com/mozilla-services/syncstorage-rs/commit/1f9323b7b3a4dd94a669099043a5692553746554), closes [#1461](https://github.com/mozilla-services/syncstorage-rs/issues/1461))

#### Chore

*   bump the rust version and some crates ([0ccaa4ed](https://github.com/mozilla-services/syncstorage-rs/commit/0ccaa4ed0205d57c12d3c86b38ffda0a27653f9d))
*   pin back to master's versions of protobuf/chrono ([e5058d26](https://github.com/mozilla-services/syncstorage-rs/commit/e5058d26d41865ec8afff93b6323c7185b16dd80))
*   fix the version.json in Docker builds (#1456) ([5f646df4](https://github.com/mozilla-services/syncstorage-rs/commit/5f646df4bb81e05885e9c097831c63e9ebede685))
*   add missing util.py to docker for process_account_events.py (#1455) ([489ee051](https://github.com/mozilla-services/syncstorage-rs/commit/489ee051a5adb5f03e0e6f30e1f9bad0018d4c39))
*   updates for Rust 1.66 (#1451) ([d1178796](https://github.com/mozilla-services/syncstorage-rs/commit/d11787965c1be802c6f07e26aa49f722f3f9cc91))
*   tag 0.13.1 (#1448) ([e48f9484](https://github.com/mozilla-services/syncstorage-rs/commit/e48f948456969f295f1250ab98156fc80e124bb8))

#### Doc

*   Minor improvements to onboarding docs (#1465) ([ef0fbfb9](https://github.com/mozilla-services/syncstorage-rs/commit/ef0fbfb9d76b4940ddb79705dcd226e34bba4401))



<a name="0.13.7"></a>
## 0.13.7 (2023-09-12)


*   Re-tag 0.13.6



<a name="0.13.6"></a>
## 0.13.6 (2023-03-07)


#### Chore

*   update tempfile crate ([670d6832](https://github.com/mozilla-services/syncstorage-rs/commit/670d68325d48f1f0f7b02e431807aa6dcd252e5f))

#### Bug Fixes

*   connect to the db once instead of every loop iteration ([31192d52](https://github.com/mozilla-services/syncstorage-rs/commit/31192d52c9677e5b5def9ffc62fd43099e499bd1))



<a name="0.13.5"></a>
## 0.13.5 (2023-03-03)


#### Bug Fixes

*   handle nullable (None) keys_changed_at values (#1464) ([7e298c2d](https://github.com/mozilla-services/syncstorage-rs/commit/7e298c2dd06dc12a0dbc2d7e6d5aab8ab8bdfba6))



<a name="0.13.4"></a>
## 0.13.4 (2023-02-24)


*   Re-tag 0.13.3



<a name="0.13.3"></a>
## 0.13.3 (2023-02-24)


#### Chore

*   add another missing file to docker for process_account_events.py (#1463) ([6ee39da4](https://github.com/mozilla-services/syncstorage-rs/commit/6ee39da4a0926e6352bf513206d1d01b63232a2e))



<a name="0.13.2"></a>
## 0.13.2 (2023-02-06)


#### Chore

*   add missing util.py to docker for process_account_events.py (#1455) (#1457) ([d2f6cf65](https://github.com/mozilla-services/syncstorage-rs/commit/d2f6cf65ff412676935e6f4306311e4599e697e9))



<a name="0.13.1"></a>
## 0.13.1 (2022-12-16)


#### Features

*   add token type to Tokenserver log lines (#1445) ([0362bcab](https://github.com/mozilla-services/syncstorage-rs/commit/0362bcab3dd057de201915b918783b0a9a2de15e), closes [#1444](https://github.com/mozilla-services/syncstorage-rs/issues/1444))

#### Bug Fixes

*   fix CORS issue (#1447) ([3f836b1e](https://github.com/mozilla-services/syncstorage-rs/commit/3f836b1e98997d98dd9671f957e5721330182b5f))

#### Chore

*   remove `spanner_config.ini` (#1446) ([b9c1f7f6](https://github.com/mozilla-services/syncstorage-rs/commit/b9c1f7f67b5e4c99642d289a0e124f1053ec54b2))
*   upgrade to Rust 1.65 (#1441) ([b95e549a](https://github.com/mozilla-services/syncstorage-rs/commit/b95e549acbf2bb31c385eb50f60016da0f54e253))



<a name="0.13.0"></a>
## 0.13.0 (2022-11-14)


#### Chore

*   temporarily disable dependabot (#1432) ([5daf6327](https://github.com/mozilla-services/syncstorage-rs/commit/5daf6327fbe4acd9f9e7acde8380e2e0d93e91bf))

#### Test

*   run the Tokenserver E2E tests without a cached JWK (#1390) ([3a18de01](https://github.com/mozilla-services/syncstorage-rs/commit/3a18de01bbf5b5c0bcb87d4176fde14840629ae2))

#### Features

*   report blocking threadpool statistics (#1418) ([929a3144](https://github.com/mozilla-services/syncstorage-rs/commit/929a3144af45b1e54e41c5c9c28c422cff0b9518), closes [#1416](https://github.com/mozilla-services/syncstorage-rs/issues/1416))
*   fix high cardinality metrics tags (#1437) ([9e36b882](https://github.com/mozilla-services/syncstorage-rs/commit/9e36b88297f387be86ac60736728ead09b9fedfc), closes [#1436](https://github.com/mozilla-services/syncstorage-rs/issues/1436))

#### Breaking Changes

*   add settings crates (#1306) ([0ae5fd20](https://github.com/mozilla-services/syncstorage-rs/commit/0ae5fd20594d3af769059088b2ff1b7899bee289), closes [#1276](https://github.com/mozilla-services/syncstorage-rs/issues/1276), breaks [#](https://github.com/mozilla-services/syncstorage-rs/issues/))

#### Refactor

*   add settings crates (#1306) ([0ae5fd20](https://github.com/mozilla-services/syncstorage-rs/commit/0ae5fd20594d3af769059088b2ff1b7899bee289), closes [#1276](https://github.com/mozilla-services/syncstorage-rs/issues/1276), breaks [#](https://github.com/mozilla-services/syncstorage-rs/issues/))



<a name="0.12.5"></a>
## 0.12.5 (2022-11-01)


#### Chore

*   switch from mariadb libmysqlclient to mysql's (#1435) ([b4fe184f](https://github.com/mozilla-services/syncstorage-rs/commit/b4fe184f5172f22bdb6885af482b658fc3368fdc), closes [#1434](https://github.com/mozilla-services/syncstorage-rs/issues/1434))



<a name="0.12.4"></a>
## 0.12.4 (2022-10-10)


#### Performance

*   always verify OAuth tokens on blocking thread (#1406) ([d69508d3](https://github.com/mozilla-services/syncstorage-rs/commit/d69508d3cc0cc9da96f7e6aab3b091495ed88346))

#### Chore

*   update to Rust 1.64.0 (#1415) ([fca795e3](https://github.com/mozilla-services/syncstorage-rs/commit/fca795e3c09c7feee12b450791a53bb0a2871b48))



<a name="0.12.3"></a>
## 0.12.3 (2022-09-23)


#### Features

*   Add `X-Content-Type-Options: nosniff` to Tokenserver responses (#1403) ([613f71ed](https://github.com/mozilla-services/syncstorage-rs/commit/613f71ed99aa875a234cbe92e1d93b6ba3413e73))



<a name="0.12.2"></a>
## 0.12.2 (2022-09-19)


#### Bug Fixes

*   fix Tokenserver generation and keys_changed_at handling (#1397) ([914e375b](https://github.com/mozilla-services/syncstorage-rs/commit/914e375b2bfa970fde01530d82e73b1af9ed3fd4))
*   don't convert all oauth token verification errors to resource_unavailable (#1389) ([ebdd609e](https://github.com/mozilla-services/syncstorage-rs/commit/ebdd609ed2ab217ed423c5b0ed9341bfbf5f73e1))



<a name="0.12.1"></a>
## 0.12.1 (2022-08-25)


#### Performance

*   remove unnecessary database calls for Tokenserver (#1360) ([5ed9a10c](https://github.com/mozilla-services/syncstorage-rs/commit/5ed9a10c2e854889a12de4f92deff106ec49d7d4))

#### Bug Fixes

*   fix Tokenserver Spanner node query (#1383) ([edef90ca](https://github.com/mozilla-services/syncstorage-rs/commit/edef90ca4795a6bbdd1b1dcaae684671097cc335))
*   fix Tokenserver node assignment query (#1382) ([9e977c71](https://github.com/mozilla-services/syncstorage-rs/commit/9e977c710ede17b3a8922b1c4a877a9dc8e93fdf))
*   fix JWT library for Tokenserver load tests (#1373) ([ebf425fe](https://github.com/mozilla-services/syncstorage-rs/commit/ebf425fe268e714c03b2e64347d71d05cc938a8b), closes [#1372](https://github.com/mozilla-services/syncstorage-rs/issues/1372))
*   rework Tokenserver load tests for local OAuth verification (#1357) ([8c59bb4f](https://github.com/mozilla-services/syncstorage-rs/commit/8c59bb4f80643d69a07c8656777a84ed9343e8e1))

#### Features

*   improve Tokenserver errors and metrics (#1385) ([90f10173](https://github.com/mozilla-services/syncstorage-rs/commit/90f101734187e159eff686dff8f89992d12b5315))
*   add `__error__` endpoint to Tokenserver (#1375) ([75231c8f](https://github.com/mozilla-services/syncstorage-rs/commit/75231c8feb996c7aa8746aeb88c9c3d428245e25), closes [#1364](https://github.com/mozilla-services/syncstorage-rs/issues/1364))
*   use Actix to spawn blocking threads (#1370) ([1b1261f2](https://github.com/mozilla-services/syncstorage-rs/commit/1b1261f23eb734b52c29862c32b3441ad70e2d5f))
*   tag Tokenserver's `token_verification` metric with the token type (#1359) ([dc00a8ea](https://github.com/mozilla-services/syncstorage-rs/commit/dc00a8ea20b3328c452880cea451789e7ab1f027), closes [#1358](https://github.com/mozilla-services/syncstorage-rs/issues/1358))



<a name="0.12.0"></a>
## 0.12.0 (2022-06-23)


#### Chore

*   add process_account_events.py to Docker image (#1325) ([75e5f273](https://github.com/mozilla-services/syncstorage-rs/commit/75e5f273abbf938730dc09af89500f1b4986fe04))
*   pin Rust 1.60.0 (#1326) ([fdc97bce](https://github.com/mozilla-services/syncstorage-rs/commit/fdc97bce4636007df3200859c4d467b29539ffd8))

#### Bug Fixes

*   fix Spanner node query (#1332) ([3e81ef14](https://github.com/mozilla-services/syncstorage-rs/commit/3e81ef14566a91ea4f89a1699090367f9450cabd), closes [#1331](https://github.com/mozilla-services/syncstorage-rs/issues/1331))
*   convert `DbError`s to `TokenserverError`s (#1327) ([9bea3280](https://github.com/mozilla-services/syncstorage-rs/commit/9bea32803cfd8f98dd7715d493cdf45ff0d54cf8), closes [#1316](https://github.com/mozilla-services/syncstorage-rs/issues/1316))
*   Set default CORS values, including all origins (#1308) ([221705b7](https://github.com/mozilla-services/syncstorage-rs/commit/221705b7ea74c6dddffd1c5289c53b3ad2cc7522))
*   write to the new version.json location (#1344) ([2821b80e](https://github.com/mozilla-services/syncstorage-rs/commit/2821b80e1bbbf3aecae74062f904739cfb6d23b2), closes [#1343](https://github.com/mozilla-services/syncstorage-rs/issues/1343))

#### Features

*   fail the health check after SYNC_LBHEARTBEAT_TTL elapses (#1337) ([a72912b8](https://github.com/mozilla-services/syncstorage-rs/commit/a72912b8757ddafd9207fec2b28d1a44975970e4), closes [#1330](https://github.com/mozilla-services/syncstorage-rs/issues/1330))
*   support multiple FxA JWKs to ease key rotation (#1339) ([eba35662](https://github.com/mozilla-services/syncstorage-rs/commit/eba3566225855119572fa840d98cb932cd603799))
*   support setting JWK for Tokenserver OAuth verification (#1307) ([d62db9f0](https://github.com/mozilla-services/syncstorage-rs/commit/d62db9f08e3e081b0b6584904d31f92ce6db273c))

#### Refactor

*   convert actix web middleware to async await (#1338) ([f76b5fc6](https://github.com/mozilla-services/syncstorage-rs/commit/f76b5fc675ebbec994618513d989c200c72ac666))
*   replaced dbg! with trace macro (#1314) ([03c059cd](https://github.com/mozilla-services/syncstorage-rs/commit/03c059cd9ed67f4bdbc6db9a09929cfa5551ea22))
*   add common crates (#1281) ([a52900f6](https://github.com/mozilla-services/syncstorage-rs/commit/a52900f6a944300371221f5beaa1f02151ce6a10), closes [#1275](https://github.com/mozilla-services/syncstorage-rs/issues/1275))



<a name="0.11.1"></a>
## 0.11.1 (2022-05-05)


#### Bug Fixes

*   to_spanner_value -> into_spanner_value (#1301) ([b8858cea](https://github.com/mozilla-services/syncstorage-rs/commit/b8858cea0756ee5920680ccd99133687c340200f), closes [#1300](https://github.com/mozilla-services/syncstorage-rs/issues/1300))



<a name="0.11.0"></a>
## 0.11.0 (2022-04-30)


#### Bug Fixes

*   fix metrics and BrowserID error context (#1294) ([a086a118](https://github.com/mozilla-services/syncstorage-rs/commit/a086a118445233c31ddd136feac74c207d707dd3))
*   fix Tokenserver migrations (#1282) ([4c64c1ce](https://github.com/mozilla-services/syncstorage-rs/commit/4c64c1ce7077dcbc67752448a1591fea0291c781))
*   add missing Tokenserver headers (#1243) ([38de8332](https://github.com/mozilla-services/syncstorage-rs/commit/38de8332a57f54607e69303433067336e85a83af), closes [#1242](https://github.com/mozilla-services/syncstorage-rs/issues/1242))
*   fix Tokenserver metrics (#1218) ([d2dc0063](https://github.com/mozilla-services/syncstorage-rs/commit/d2dc0063ed336c339f5668a2154def1ada96af75), closes [#1214](https://github.com/mozilla-services/syncstorage-rs/issues/1214))
*   move I/O calls to blocking threadpool (#1190) ([cbeebf46](https://github.com/mozilla-services/syncstorage-rs/commit/cbeebf465ae5f87719de0335fefe232741acd1a3), closes [#1188](https://github.com/mozilla-services/syncstorage-rs/issues/1188))
*   resolve intermittent Tokenserver test failure (#1171) ([0c05e999](https://github.com/mozilla-services/syncstorage-rs/commit/0c05e999d1ceab8942d313d741a0a6eee4d5a117), closes [#1170](https://github.com/mozilla-services/syncstorage-rs/issues/1170))
*   Revert "update sentry version and remove ignore rustsec-2020-0041 in … (#1137) ([48947bbf](https://github.com/mozilla-services/syncstorage-rs/commit/48947bbf8a6ed6a47c0f7725451869451bdf1cb2))
*   removed send from async_trait for DbPool (#1139) ([8c603de6](https://github.com/mozilla-services/syncstorage-rs/commit/8c603de683f292b88f92bd6621110cf212424d1b))
*   Fix build for Rust 1.53.0 (#1106) ([0b37bbe0](https://github.com/mozilla-services/syncstorage-rs/commit/0b37bbe076a0d02808ae5387ab6d334f7dfdaf2a), closes [#1105](https://github.com/mozilla-services/syncstorage-rs/issues/1105))
*   Convert integral values to String before converting to Value (#1056) ([21da763b](https://github.com/mozilla-services/syncstorage-rs/commit/21da763b144e2a146b859d7b7ff579aa067bb150), closes [#1055](https://github.com/mozilla-services/syncstorage-rs/issues/1055))
*   use ValidationErrorKind metric_label in ApiError  (#1038) ([4dc77afd](https://github.com/mozilla-services/syncstorage-rs/commit/4dc77afd5115cffc04168f3615126222ff180f4f))

#### Chore

*   disable grpcio openssl (#1288) ([8ff7a40d](https://github.com/mozilla-services/syncstorage-rs/commit/8ff7a40de715da18e24cb047b0064ea437eac390))
*   prefer CIRCLE_SHA1 vs CIRCLE_TAG in circle's cache key (#1285) ([37d2251c](https://github.com/mozilla-services/syncstorage-rs/commit/37d2251c82d7ec73053ee39500466ba9c5edf19b), closes [#1284](https://github.com/mozilla-services/syncstorage-rs/issues/1284))
*   update for Rust 1.60.0 (#1280) ([c4bca395](https://github.com/mozilla-services/syncstorage-rs/commit/c4bca395f1dc61184186df98b63ab419b44361bf))
*   add Python build to Makefile (#1244) ([291a40ea](https://github.com/mozilla-services/syncstorage-rs/commit/291a40eaa49b583349ba07ee155a214b22d76e01), closes [#1226](https://github.com/mozilla-services/syncstorage-rs/issues/1226))
*   update regex (#1252) ([fc34353a](https://github.com/mozilla-services/syncstorage-rs/commit/fc34353a0aba96c4e1bb2d8fe9b6d8d8335058b9))
*   update to Rust 1.59 (#1227) ([0e9b0f6e](https://github.com/mozilla-services/syncstorage-rs/commit/0e9b0f6e6c61b78a2ef2b1c3a32b9a73850b391b))
*   update to Rust 1.58 and switch to GCP Rust crate (#1201) ([a7c5f809](https://github.com/mozilla-services/syncstorage-rs/commit/a7c5f809d03d62cc27c77d919dbe85e2d63bde64))
*   label the circleci e2e tests (#1185) ([bf3ef8b3](https://github.com/mozilla-services/syncstorage-rs/commit/bf3ef8b31aefdc9c9544aa8eccd1e82ed6562198))
*   update 12/2/21 (#1181) ([04cf2344](https://github.com/mozilla-services/syncstorage-rs/commit/04cf2344d56c2697dcf45d0caacd3cfd5a8b2bb6))
*   update actix-http version due to RUSTSEC-2021-0081 (#1140) ([0106131e](https://github.com/mozilla-services/syncstorage-rs/commit/0106131e87a7bbe8a27099783f7eccbc8112c47d))
*   switch failure crate with thiserror (#1122) ([5369f1ae](https://github.com/mozilla-services/syncstorage-rs/commit/5369f1aef4a8dfa0fc22dfedeb5aa10af8bf3186))
*   Update code for Rust 1.54.0 (#1123) ([7ab37291](https://github.com/mozilla-services/syncstorage-rs/commit/7ab37291450dc6ba6a40bf6fc7503732a4a3f617))
*   enable flake8 in circleci config for tools/integration_tests (#1121) ([dee69dd3](https://github.com/mozilla-services/syncstorage-rs/commit/dee69dd33da8ba1022023d18c8414675427ca12f))
*   Updates for May 2021 (#1078) ([f25e4e0f](https://github.com/mozilla-services/syncstorage-rs/commit/f25e4e0fae478cd82604b126782889a31fc0cac1))
*   tag 0.10.1 (#1042) ([ecada4b3](https://github.com/mozilla-services/syncstorage-rs/commit/ecada4b3b07f2a22902d3b0117e65202ab3a22f9))

#### Test

*   Add BrowserId support to Tokenserver load tests (#1219) ([b6d87b72](https://github.com/mozilla-services/syncstorage-rs/commit/b6d87b7214a2d7cf54563aa0d357539c6e3b863b), closes [#1213](https://github.com/mozilla-services/syncstorage-rs/issues/1213))
*   add Tokenserver load tests (#1184) ([46d4a9ea](https://github.com/mozilla-services/syncstorage-rs/commit/46d4a9ea431a120fbf1626e4193f9f9b2b98d928), closes [#1107](https://github.com/mozilla-services/syncstorage-rs/issues/1107))
*   add Tokenserver integration tests to CI (#1180) ([aa18c1a0](https://github.com/mozilla-services/syncstorage-rs/commit/aa18c1a01db2167319303cc6af0353e1e383861e), closes [#1174](https://github.com/mozilla-services/syncstorage-rs/issues/1174))
*   Add Tokenserver integration tests (#1152) ([7209ccf5](https://github.com/mozilla-services/syncstorage-rs/commit/7209ccf551fc35228221dc8739cc3419ef9afbcb), closes [#1048](https://github.com/mozilla-services/syncstorage-rs/issues/1048))

#### Doc

*   add Tokenserver README (#1162) ([b5fa8c8a](https://github.com/mozilla-services/syncstorage-rs/commit/b5fa8c8a8926166af3146ea484b2f813b7dc4d13), closes [#1082](https://github.com/mozilla-services/syncstorage-rs/issues/1082))
*   add comments about Tokenserver state being an Option (#1161) ([c1dc552b](https://github.com/mozilla-services/syncstorage-rs/commit/c1dc552b1cbd190d127444dd728d1071e238f6a5), closes [#1102](https://github.com/mozilla-services/syncstorage-rs/issues/1102))
*   Add Apache 2.0 license to prepare-spanner.sh (#1120) ([f0c16ba5](https://github.com/mozilla-services/syncstorage-rs/commit/f0c16ba5c8a36dcd8d0fa9ff8a2bec2b36aa9c96))
*   Update spanner configuration documentation (#1047) ([57405c1e](https://github.com/mozilla-services/syncstorage-rs/commit/57405c1edd5a845c05e653b22f98642b524466a2), closes [#1045](https://github.com/mozilla-services/syncstorage-rs/issues/1045))

#### Refactor

*   cache FxA OAuth client (#1212) ([04b24378](https://github.com/mozilla-services/syncstorage-rs/commit/04b2437816b2b653f0143fa333d1e61230466cb3), closes [#1209](https://github.com/mozilla-services/syncstorage-rs/issues/1209))
*   Remove Tokenserver support for per-node secrets (#1211) ([eac6b558](https://github.com/mozilla-services/syncstorage-rs/commit/eac6b55889b1a42abb495baa45148ebdee55e185), closes [#1208](https://github.com/mozilla-services/syncstorage-rs/issues/1208))
*   remove static service IDs (#1199) ([ae659702](https://github.com/mozilla-services/syncstorage-rs/commit/ae6597022c7efcc0597c353f49da88289815074a), closes [#1144](https://github.com/mozilla-services/syncstorage-rs/issues/1144), breaks [#](https://github.com/mozilla-services/syncstorage-rs/issues/))
*   Tokenserver: Add mature MySQL adapter (#1119) ([503d1aa8](https://github.com/mozilla-services/syncstorage-rs/commit/503d1aa81bb99d2647565464360df57e00f028a0), closes [#1054](https://github.com/mozilla-services/syncstorage-rs/issues/1054))
*   Use PyFxA to verify Tokenserver tokens (#1103) ([34401777](https://github.com/mozilla-services/syncstorage-rs/commit/34401777027bd756f57ac35a7937ad8dbc972121), closes [#1102](https://github.com/mozilla-services/syncstorage-rs/issues/1102))
*   Clean up Tokenserver code (#1087) ([e9247699](https://github.com/mozilla-services/syncstorage-rs/commit/e9247699180c4d27431a4b5916bbab587a3f159e), closes [#968](https://github.com/mozilla-services/syncstorage-rs/issues/968))
*   Tokenserver: Rewrite inlined Python code in Rust (#1053) ([34fe5859](https://github.com/mozilla-services/syncstorage-rs/commit/34fe5859e6d6e2a67b745bbbc6480b99ce6ba343), closes [#1049](https://github.com/mozilla-services/syncstorage-rs/issues/1049))
*   Add ToSpannerValue trait (#1046) ([2ce45705](https://github.com/mozilla-services/syncstorage-rs/commit/2ce45705f7bfa4ae0b968c41ccbef9f70c1352ac), closes [#260](https://github.com/mozilla-services/syncstorage-rs/issues/260))
*   Emit metric for spanner DbPool::get time taken (#1044) ([57bd30ad](https://github.com/mozilla-services/syncstorage-rs/commit/57bd30ad39d43acdfa3cbe2abb00917f89860b69))
*   Use generic tuple extractor in web extractors (#1043) ([71c62be1](https://github.com/mozilla-services/syncstorage-rs/commit/71c62be1ac74b0642b150c16cd79eff0123261f4), closes [#698](https://github.com/mozilla-services/syncstorage-rs/issues/698))

#### Breaking Changes

*   remove static service IDs (#1199) ([ae659702](https://github.com/mozilla-services/syncstorage-rs/commit/ae6597022c7efcc0597c353f49da88289815074a), closes [#1144](https://github.com/mozilla-services/syncstorage-rs/issues/1144), breaks [#](https://github.com/mozilla-services/syncstorage-rs/issues/))

#### Features

*   spawn Tokenserver pool reporter (#1283) ([ee8e1794](https://github.com/mozilla-services/syncstorage-rs/commit/ee8e17947912a7798db3a85d06abae9dbbad7d05))
*   don't run Tokenserver migrations on startup (#1286) ([1a197a6c](https://github.com/mozilla-services/syncstorage-rs/commit/1a197a6c6e359b6915a7b357c56a7061c80bb8d4))
*   emit 4XX errors as metrics instead of Sentry events (#1274) ([cacd8285](https://github.com/mozilla-services/syncstorage-rs/commit/cacd8285048fe7fed91a2958222e34f057e420c3))
*   add context to `TokenserverError`s (#1224) ([92e7d262](https://github.com/mozilla-services/syncstorage-rs/commit/92e7d262076a04191df5d85b2167d25a0f62dd61), closes [#1223](https://github.com/mozilla-services/syncstorage-rs/issues/1223))
*   Pass Tokenserver origin field through token payload (#1264) ([a4c340e1](https://github.com/mozilla-services/syncstorage-rs/commit/a4c340e194804b9531558de9263aeb67351b16f2), closes [#1245](https://github.com/mozilla-services/syncstorage-rs/issues/1245))
*   add BrowserID support for Tokenserver (#1216) ([38d6a27b](https://github.com/mozilla-services/syncstorage-rs/commit/38d6a27b02e9ded7ef279a33f3a562e08e72f6a8), closes [#1215](https://github.com/mozilla-services/syncstorage-rs/issues/1215))
*   add Tokenserver metrics (#1200) ([aa93312a](https://github.com/mozilla-services/syncstorage-rs/commit/aa93312a1c1e7c1e102ad38a1ff935518e437cb4), closes [#1108](https://github.com/mozilla-services/syncstorage-rs/issues/1108))
*   add missing Tokenserver response fields (#1176) ([a3d4f094](https://github.com/mozilla-services/syncstorage-rs/commit/a3d4f094cd11159c95d2068468200c33a4e2f294), closes [#1173](https://github.com/mozilla-services/syncstorage-rs/issues/1173))
*   Tokenserver: add per-node secrets (#1169) ([bed59e2c](https://github.com/mozilla-services/syncstorage-rs/commit/bed59e2cb7d6d69b48c35f223ab6cbaf756109ea), closes [#1104](https://github.com/mozilla-services/syncstorage-rs/issues/1104))
*   make Tokenserver DbPool#get async (#1175) ([3d4c180d](https://github.com/mozilla-services/syncstorage-rs/commit/3d4c180d34b38455cf6c5022ce28661e83e5addf), closes [#1172](https://github.com/mozilla-services/syncstorage-rs/issues/1172))
*   add Tokenserver admin scripts (#1168) ([0ac30958](https://github.com/mozilla-services/syncstorage-rs/commit/0ac30958de5cfca0f3d44dfb479b615cae7ede27), closes [#1086](https://github.com/mozilla-services/syncstorage-rs/issues/1086))
*   Add client state validation (#1160) ([0996cb15](https://github.com/mozilla-services/syncstorage-rs/commit/0996cb154fd7d334f2dd6fc6603557774fd1374b), closes [#1091](https://github.com/mozilla-services/syncstorage-rs/issues/1091))
*   Tokenserver: Add node assignment logic (#1158) ([db739def](https://github.com/mozilla-services/syncstorage-rs/commit/db739defbe180bddc4f61c3c796ff8b328c84a64), closes [#1051](https://github.com/mozilla-services/syncstorage-rs/issues/1051))
*   Tokenserver: Add validations and user updating for generation, keys_changed_at, and client_state (#1145) ([337ab8f4](https://github.com/mozilla-services/syncstorage-rs/commit/337ab8f406a23b44f3b173ecf06ba2caeca571dc), closes [#866](https://github.com/mozilla-services/syncstorage-rs/issues/866))
*   Add ability to disable syncstorage endpoints (#1159) ([5f2fa8a3](https://github.com/mozilla-services/syncstorage-rs/commit/5f2fa8a35d9c2dba09fd302d69d002b525918b04), closes [#1083](https://github.com/mozilla-services/syncstorage-rs/issues/1083))
*   Tokenserver: Add support for client-specified token duration (#1151) ([17f89ac5](https://github.com/mozilla-services/syncstorage-rs/commit/17f89ac5f0b1754265828697075261075a3e8f25), closes [#1050](https://github.com/mozilla-services/syncstorage-rs/issues/1050))
*   Add test mode to Tokenserver (#1143) ([cbdc12e5](https://github.com/mozilla-services/syncstorage-rs/commit/cbdc12e5306b40e426fbfc303913e3aff4330e08), closes [#1142](https://github.com/mozilla-services/syncstorage-rs/issues/1142))
*   Tokenserver: Implement extractors for generation, keys_changed_at, client_state (#1141) ([f29064d3](https://github.com/mozilla-services/syncstorage-rs/commit/f29064d3be79181ec3fa7c2399bbf0c2a272101c), closes [#1133](https://github.com/mozilla-services/syncstorage-rs/issues/1133))
*   generation, keys_changed_at, and client_state: Add database methods (#1136) ([44e832b8](https://github.com/mozilla-services/syncstorage-rs/commit/44e832b8dbcf3ad31f74e02a66d03372f0dee540), closes [#1132](https://github.com/mozilla-services/syncstorage-rs/issues/1132))
*   Integrate Spanner emulator with CI (#1079) ([e6ec1acd](https://github.com/mozilla-services/syncstorage-rs/commit/e6ec1acd8742fab3c456e0dc948ea4c8cc21864d), closes [#566](https://github.com/mozilla-services/syncstorage-rs/issues/566))
*   Support SYNC_SPANNER_EMULATOR_HOST (#1061) ([322603a7](https://github.com/mozilla-services/syncstorage-rs/commit/322603a7fec6c0ccefedc4298ab79040f9ccfdc6), closes [#915](https://github.com/mozilla-services/syncstorage-rs/issues/915))



<a name="0.10.2"></a>
## 0.10.2 (2021-04-28)


#### Bug Fixes

*   update deadpool w/ the incorrect pool stats fix (#1057) ([d261ac1e](https://github.com/mozilla-services/syncstorage-rs/commit/d261ac1ebcc1ed3ff2871e5fd61ab4a934149fcd), closes [#803](https://github.com/mozilla-services/syncstorage-rs/issues/803))



<a name="0.10.1"></a>
## 0.10.1 (2021-04-14)


#### Refactor

*   Remove middleware::sentry::queue_report (#1040) ([0dccb00f](https://github.com/mozilla-services/syncstorage-rs/commit/0dccb00fb95d0aebabe79d5e6ecb1fb537445444))



<a name="0.10.0"></a>
## 0.10.0 (2021-04-05)


#### Bug Fixes

*   Restore hawk error metrics (#1033) ([f795eb08](https://github.com/mozilla-services/syncstorage-rs/commit/f795eb0813b4ee37463add5391c829c906fdb35d), closes [#812](https://github.com/mozilla-services/syncstorage-rs/issues/812))
*   report query parameters with Invalid Value error (#1030) ([354cf794](https://github.com/mozilla-services/syncstorage-rs/commit/354cf794c59266dccfd3c6d12b880b466efa5650))

#### Features

*   Add "auto-split" arg to auto-gen UID prefixes (#1035) ([487ac11e](https://github.com/mozilla-services/syncstorage-rs/commit/487ac11ed0abf4ddc77cea1be852169846796a57))



<a name="0.9.1"></a>
## 0.9.1 (2021-03-12)


#### Chore

*   kill the long unused db middleware ([8f9cce76](https://github.com/mozilla-services/syncstorage-rs/commit/8f9cce76ba4a52e4594f32b471f2e0259abe04d2), closes [#693](https://github.com/mozilla-services/syncstorage-rs/issues/693), [#1018](https://github.com/mozilla-services/syncstorage-rs/issues/1018))
*   Update for March 2021 ([4e38e681](https://github.com/mozilla-services/syncstorage-rs/commit/4e38e68180766c083b651d148c24f42e5d0fd058), closes [#1018](https://github.com/mozilla-services/syncstorage-rs/issues/1018))

#### Features

*   Manually update the spanner session approximate_last_used_time (#1009) ([f669b257](https://github.com/mozilla-services/syncstorage-rs/commit/f669b257a2d8b8f4928a32df27eebe33f1af555e), closes [#1008](https://github.com/mozilla-services/syncstorage-rs/issues/1008))



<a name="0.9.0"></a>
## 0.9.0 (2021-02-25)


#### Bug Fixes

*   ensure "extra" data added to Hawk header in e2e tests (#1003) ([8c280ccd](https://github.com/mozilla-services/syncstorage-rs/commit/8c280ccda032ff007c4a6017d6fb0cdd94d7cd3f))

#### Features

*   kill old or excessively idled connections (#1006) ([082dd1f0](https://github.com/mozilla-services/syncstorage-rs/commit/082dd1f0613fc91f3ea2f02b3bcdd9ddf2b938d3))

#### Chore

*   fmt & clippy fixes for Rust 1.50 (#1004) ([56cadcb2](https://github.com/mozilla-services/syncstorage-rs/commit/56cadcb2cdcce99de2d425e8d0edb4a7c20c52ac))
*   RUSTSEC-2021-0020 fix ([2e186341](https://github.com/mozilla-services/syncstorage-rs/commit/2e1863410ed92180f6fb12a9ebf8d2f462425b38), closes [#999](https://github.com/mozilla-services/syncstorage-rs/issues/999))
*   tag 0.8.7 (#998) ([d06b3c2a](https://github.com/mozilla-services/syncstorage-rs/commit/d06b3c2a0dd1602d074d5d2da913db699eea0a9e))



<a name="0.8.7"></a>
### 0.8.7  (2021-02-03)


#### Features

*   `__lbheartbeat__` will return 500 if the connection pool is exhausted (#997) ([e72573ac](https://github.com/mozilla-services/syncstorage-rs/commit/e72573acedce2916c9fd3aa8e3c54fbe71f2008e), closes [#996](https://github.com/mozilla-services/syncstorage-rs/issues/996))

#### Chore

*   tag 0.8.6 (#995) ([8cb5b603](https://github.com/mozilla-services/syncstorage-rs/commit/8cb5b603f0320483904107eee682797b8d814a44))



<a name="0.8.6"></a>
### 0.8.6 (2021-02-01)


#### Refactor

*   remove duplicate code for incrementing counters (#983) ([d72228b1](https://github.com/mozilla-services/syncstorage-rs/commit/d72228b1d4b5cd63a399bde77c3156ea53bb4217))

#### Chore

*   tag 0.8.5 (#979) ([3c23fb46](https://github.com/mozilla-services/syncstorage-rs/commit/3c23fb46138d4a042d0293af6b9853ea9f173f6d))

#### Features

*   Add pool connection info to __lbheartbeat__ for ops (#985) ([06a2ac79](https://github.com/mozilla-services/syncstorage-rs/commit/06a2ac7910a87a75f1a2f0d68e786579cec99fd8))

#### Bug Fixes

*   downgrade deadpool so it stays on tokio 0.2 ([99975ef8](https://github.com/mozilla-services/syncstorage-rs/commit/99975ef8b64317511111d48c6ebfc75e7facc334), closes [#976](https://github.com/mozilla-services/syncstorage-rs/issues/976))



<a name="0.8.5"></a>
## 0.8.5 (2021-01-21)


#### Bug Fixes

*   downgrade deadpool so it stays on tokio 0.2 ([99975ef8](https://github.com/mozilla-services/syncstorage-rs/commit/99975ef8b64317511111d48c6ebfc75e7facc334), closes [#976](https://github.com/mozilla-services/syncstorage-rs/issues/976))



<a name="0.8.4"></a>
### 0.8.4 (2021-01-19)


#### Chore
*   Update pyo3 to the latest version (#938) ([cc7d9d36]https://github.com/mozilla-services/syncstorage-rs/commit/cc7d9d367310aeb7551668c049f1a895a6eae853))
*   update dependencies (#953) ([bca8770f](https://github.com/mozilla-services/syncstorage-rs/commit/bca8770f531b45b00e57e137082b1ed9d90acd7f))
*   tag 0.8.3 (#937) ([02b76231](https://github.com/mozilla-services/syncstorage-rs/commit/02b76231cf4fa015093cea75286a82f306c833b0))


#### Features

*   default to timing out deadpool checkouts (30 seconds) (#974) ([2ecca202](https://github.com/mozilla-services/syncstorage-rs/commit/2ecca202aa01f123898115827af6e5967f8a1e9b), closes [#973](https://github.com/mozilla-services/syncstorage-rs/issues/973))
*   avoid an unnecessarily cloning for from_raw_bso (#972) ([07352b6d](https://github.com/mozilla-services/syncstorage-rs/commit/07352b6d7a331d07e18ec386a650d3b720c5703f), closes [#971](https://github.com/mozilla-services/syncstorage-rs/issues/971))

<a name="0.8.3"></a>
### 0.8.3 (2020-11-30)


#### Chore

*   Update to rust 1.48 (#927) ([ea1f222b](https://github.com/mozilla-services/syncstorage-rs/commit/ea1f222b219ddd78684945058c3b3430ed636982))

<a name="0.8.2"></a>
## 0.8.2 (2020-11-20)


#### Bug Fixes

*   make actix-cors more permissive (#929) ([1a7e817a](https://github.com/mozilla-services/syncstorage-rs/commit/1a7e817a15d2ad0cb4a979e114cbcfa074402314))



<a name="0.8.1"></a>
### 0.8.1 (2020-11-16)


#### Chore

*   Update depenedencies (#904) ([4e95c571](https://github.com/mozilla-services/syncstorage-rs/commit/4e95c571c73953e1f92bee46a58c49a97d9aa463), closes [#899](https://github.com/mozilla-services/syncstorage-rs/issues/899))
*   update dependencies (#900) ([0afb9691](https://github.com/mozilla-services/syncstorage-rs/commit/0afb9691f7538dd9eaa68dc7eac11a2e06a12a70))
*   tag 0.8.0 (#881) ([b6ff73d2](https://github.com/mozilla-services/syncstorage-rs/commit/b6ff73d2916e5c4afacede8bb2db905a576dba26))
*   tag 0.8.1 for release and include scripts for setting up sentry releases (#881) ([33f900dc8e](https://github.com/mozilla-services/syncstorage-rs/commit/33f900dc8edd4e583b04af9363ef2cc51a0c889d))


#### Test

*   add a basic overquota test (#912) ([5afda742](https://github.com/mozilla-services/syncstorage-rs/commit/5afda7427b487110cc256cda4f517e8ea2f796fb), closes [#120](https://github.com/mozilla-services/syncstorage-rs/issues/120))


#### Features

*   Add `SYNC_ENFORCE_QUOTA` flag (#875) ([0e30801d](https://github.com/mozilla-services/syncstorage-rs/commit/0e30801dbbfe3693c8d2c21c0e6fc09262d7afb3), closes [#870](https://github.com/mozilla-services/syncstorage-rs/issues/870))
*   switch coll cache's RwLock to async (#906) ([14fc49a5](https://github.com/mozilla-services/syncstorage-rs/commit/14fc49a559e69c695bc17c220b72817b2d971e1d), closes [#905](https://github.com/mozilla-services/syncstorage-rs/issues/905))
*   Implement rudimentary tokenserver route in syncstorage-rs (#871) ([b74943e4](https://github.com/mozilla-services/syncstorage-rs/commit/b74943e4580e0db36f3a1a55c2eb8f9083f2759b))

#### Bug Fixes

*   downgrade to sentry 0.19 ([243eb17a](https://github.com/mozilla-services/syncstorage-rs/commit/243eb17a35ce3dc1c07090dcf0439e4eadeb855a), closes [#907](https://github.com/mozilla-services/syncstorage-rs/issues/907))
*   add a short delay to avoid 503s (#922) ([36698137](https://github.com/mozilla-services/syncstorage-rs/pull/922/commits/ecf073e300630c56e0659c9bbb00653c442937f4), closes [#920](https://github.com/mozilla-services/syncstorage-rs/issues/920))



<a name="0.8.0"></a>
## 0.8.0 (2020-10-29)


#### Bug Fixes

*   handle duplicate keys in batch_upload_items for mysql (#873) ([2d6039f3](https://github.com/mozilla-services/syncstorage-rs/commit/2d6039f3e6b130a3a45c6c1815c1bcc25279d451), closes [#827](https://github.com/mozilla-services/syncstorage-rs/issues/827))
*   reduce MAX_TOTAL_RECORDS for quota write allowance ([bac2c51f](https://github.com/mozilla-services/syncstorage-rs/commit/bac2c51f4f44289d982c69437b5e803948a6a1b7))
*   avoid extra quota related work in batch commit ([51c3bdab](https://github.com/mozilla-services/syncstorage-rs/commit/51c3bdab9988dcbfa59d10af5cda81335f71a270), closes [#869](https://github.com/mozilla-services/syncstorage-rs/issues/869))
*   correct quota env var in config test to SYNC_ENABLE_QUOTA (#859) ([f0aa4642](https://github.com/mozilla-services/syncstorage-rs/commit/f0aa4642b13a9e4d687707940959cc181e6f750d), closes [#829](https://github.com/mozilla-services/syncstorage-rs/issues/829))

#### Chore

*   tag 0.7.1 (#863) ([0400863e](https://github.com/mozilla-services/syncstorage-rs/commit/0400863e89589933c62fbaef0188f18970a53d9d))

#### Features

*   Add `count` and `count_with_tags` metric for batch histogram (#879) ([8afcbe65](https://github.com/mozilla-services/syncstorage-rs/commit/8afcbe65de944c5ef3cd579f0891dba7bc403e71), closes [#878](https://github.com/mozilla-services/syncstorage-rs/issues/878))
*   optimize POST w/ ?batch=true&commit=true (#880) ([b7e9ba53](https://github.com/mozilla-services/syncstorage-rs/commit/b7e9ba535308721a1312e774317f3aff170a7520), closes [#876](https://github.com/mozilla-services/syncstorage-rs/issues/876))
*   remove Tags handoffs (#862) ([c6ea474c](https://github.com/mozilla-services/syncstorage-rs/commit/c6ea474c16ac003395d10c6b282c84050cfece6c), closes [#403](https://github.com/mozilla-services/syncstorage-rs/issues/403))
*   rework error logging/metric reporting; fix BSO batch updates for spanner (#824) ([cef8fb52](https://github.com/mozilla-services/syncstorage-rs/commit/cef8fb521ad3239f5ecf356468715ca8341e7f73), closes [#827](https://github.com/mozilla-services/syncstorage-rs/issues/827))



<a name="0.7.1"></a>
## 0.7.1 (2020-10-19)


#### Bug Fixes

*   correct quota env var in config test to SYNC_ENABLE_QUOTA (#859) ([f0aa4642](https://github.com/mozilla-services/syncstorage-rs/commit/f0aa4642b13a9e4d687707940959cc181e6f750d), closes [#829](https://github.com/mozilla-services/syncstorage-rs/issues/829))
* rework error logging/metric reporting; fix BSO batch updates for spanner (#174, #619, #618) ([cef8fb521](https://github.com/mozilla-services/syncstorage-rs/commit/cef8fb521ad3239f5ecf356468715ca8341e7f73), closes [#174](https://github.com/mozilla-services/syncstorage-rs/issues/174), [#619](https://github.com/mozilla-services/syncstorage-rs/issues/619), [#618](https://github.com/mozilla-services/syncstorage-rs/issues/618))



<a name="0.7.0"></a>
## 0.7.0 (2020-10-12)


#### Bug Fixes

*   Return FORBIDDEN if a user's batch is Over Quota (#848) ([d24dcdb6](https://github.com/mozilla-services/syncstorage-rs/commit/d24dcdb6c1a23ea725322830b82a3f31a11c7a8b), closes [#852](https://github.com/mozilla-services/syncstorage-rs/issues/852))
*   clippy error related to matches! closes #850 ([06aed80f](https://github.com/mozilla-services/syncstorage-rs/commit/06aed80f004c355f280d25c9d508b28038adf0f2))
*   downgrade sentry to 0.19 (#849) ([0a175dde](https://github.com/mozilla-services/syncstorage-rs/commit/0a175dde049b4661d681be5398941f6a3136a142))

#### Chore

*   Update circleci to use docker auth (#855) ([dcb0a0b2](https://github.com/mozilla-services/syncstorage-rs/commit/dcb0a0b23c78b5f07c0a8f4c2d91f4f5895a7515), closes [#854](https://github.com/mozilla-services/syncstorage-rs/issues/854))
*   update to protobuf 2.18.0 ([c6f9cf9b](https://github.com/mozilla-services/syncstorage-rs/commit/c6f9cf9bd4ef7bff13ddc33a71f5771dd9bf6ea3), closes [#852](https://github.com/mozilla-services/syncstorage-rs/issues/852))



<a name="0.6.1"></a>
## 0.6.1 (2020-09-30)

#### Features
* update to actix-web 3 (#834)

#### Bug Fixes
* return correct error code and value for OverQuota users (#837)


<a name="0.6.0"></a>
## 0.6.0 (2020-09-23)

#### Features
*   Add quota monitoring (#806) ([9e6759e](https://github.com/mozilla-services/syncstorage-rs/commit/9e6759efef8f163355ee1b23dc28b716335df66d), closes [#791](https://github.com/mozilla-services/syncstorage-rs/issues/791), [#793](https://github.com/mozilla-services/syncstorage-rs/issues/793), [#797](https://github.com/mozilla-services/syncstorage-rs/issues/797), [#789](https://github.com/mozilla-services/syncstorage-rs/issues/789), [#801](https://github.com/mozilla-services/syncstorage-rs/issues/801))
*   Convert some of the validation storage errors into metrics (#810) ([66221d8b](https://github.com/mozilla-services/syncstorage-rs/commit/66221d8bec17f6134dee1b9d9005f5cdbe8121d3), closes [#795](https://github.com/mozilla-services/syncstorage-rs/issues/795))
*   switch from `regex_contains` to `starts_with` (#805) ([a79f8407](https://github.com/mozilla-services/syncstorage-rs/commit/a79f8407de7b5f01413b09771dcfa8bb8e33ab9e))



<a name="0.5.8"></a>
## 0.5.8 (2020-08-25)


#### Bug Fixes

*   fix purge_ttl advanced features ([714168d1](https://github.com/mozilla-services/syncstorage-rs/commit/714168d1077e3429bd33fbcb17724cd74551149a), closes [#799](https://github.com/mozilla-services/syncstorage-rs/issues/799))

#### Features

*   cleanup the spanner pool managers ([746f5d12](https://github.com/mozilla-services/syncstorage-rs/commit/746f5d128f3d3804367b49b6b5fbff34e722d5b3), closes [#794](https://github.com/mozilla-services/syncstorage-rs/issues/794))



<a name="0.5.7"></a>
## 0.5.7 (2020-08-22)


#### Chore

*   update protobuf to 2.17.0 (#783) ([af5234d4](https://github.com/mozilla-services/syncstorage-rs/commit/af5234d4ceb9db479e550d06796d783d4cec33aa), closes [#782](https://github.com/mozilla-services/syncstorage-rs/issues/782))

#### Bug Fixes

*   Avoid implicit transactions in DbTransactionPool (#777) ([e0448583](https://github.com/mozilla-services/syncstorage-rs/commit/e044858323297a95bcc903c7bc983b9093422fc7), closes [#768](https://github.com/mozilla-services/syncstorage-rs/issues/768))

#### Features

*   switch spanner's db pool to deadpool ([077bf091](https://github.com/mozilla-services/syncstorage-rs/commit/077bf091ecaededfa3c937ce5ac5a5f6f95015f3))
*   emit internal bb8 Pool errors to logs/sentry ([ec25bc47](https://github.com/mozilla-services/syncstorage-rs/commit/ec25bc47e2eed88a6fdabc3d32d04d065a780e67), closes [#786](https://github.com/mozilla-services/syncstorage-rs/issues/786), [#785](https://github.com/mozilla-services/syncstorage-rs/issues/785), [#764](https://github.com/mozilla-services/syncstorage-rs/issues/764), [#787](https://github.com/mozilla-services/syncstorage-rs/issues/787))

#### Refactor

*   cleanup/rearrange ([7e526cb8](https://github.com/mozilla-services/syncstorage-rs/commit/7e526cb831dfacce65415822841c8881b0ce771e))



<a name="0.5.6"></a>
## 0.5.6 (2020-08-11)


#### Features

*   More purge_ttl features (#776) ([59aa28a4](https://github.com/mozilla-services/syncstorage-rs/commit/59aa28a4e5fdcfe2acc3f767487066d30b998af0), closes [#735](https://github.com/mozilla-services/syncstorage-rs/issues/735), [#743](https://github.com/mozilla-services/syncstorage-rs/issues/743))

#### Bug Fixes

*   remove ubuntu target for grpcio (#775) ([7d1061f7](https://github.com/mozilla-services/syncstorage-rs/commit/7d1061f7197a56936a6cff9a438997640892d6c6), closes [#774](https://github.com/mozilla-services/syncstorage-rs/issues/774))
*   Return WeaveError::OverQuota for over quota responses (#773) ([38cd5ddd](https://github.com/mozilla-services/syncstorage-rs/commit/38cd5dddc36ae0aeda159fea88ba6128a8e85181), closes [#769](https://github.com/mozilla-services/syncstorage-rs/issues/769))
*   ensure an X-Last-Modified for /info/configuration (#761) ([36533f85](https://github.com/mozilla-services/syncstorage-rs/commit/36533f8566c39e8c82ccb5a2bc8ae62fb254129a), closes [#759](https://github.com/mozilla-services/syncstorage-rs/issues/759))



<a name="0.5.5"></a>
### 0.5.5 (2020-08-06)

#### Chore

*   Update vendored SDK to use protobuf 2.16.2 (#747) ([39519bb8](https://github.com/mozilla-services/syncstorage-rs/commit/39519bb821fdf58ecf5842c6b818a58d53167135))

#### Bug Fixes

*   set config env separator to double underscore. (#763) ([f1d88fea](https://github.com/mozilla-services/syncstorage-rs/commit/f1d88feae60d7fea15b7575ac2108f0f80ff42b4), closes [#762](https://github.com/mozilla-services/syncstorage-rs/issues/762))
*   normalize id elements to remove potential wrap characters (#748) ([71ab9b34](https://github.com/mozilla-services/syncstorage-rs/commit/71ab9b344601479de2b4ebcf3b221720577f6e74), closes [#680](https://github.com/mozilla-services/syncstorage-rs/issues/680))



<a name="0.5.4"></a>
### 0.5.4 (2020-08-04)


#### Features

*   add debug_client check to BsoBodies for batch operations. ([1370df9d](https://github.com/mozilla-services/syncstorage-rs/commit/1370df9d7c2e6d656f50332b3f8615faafacead0)



<a name="0.5.3"></a>
## 0.5.3 (2020-07-31)


#### Features

*   force client to rec'v over quota error ([81c00c31](https://github.com/mozilla-services/syncstorage-rs/commit/81c00c31b89c21d20563aef9d31a351a7d581c3c), closes [#746](https://github.com/mozilla-services/syncstorage-rs/issues/746))
*   add metric for db conflicts ([1595f27f](https://github.com/mozilla-services/syncstorage-rs/commit/1595f27f4d4061c610078cb569790a1bdc52fc50))

#### Bug Fixes

*   defer grpc auth to actix-web's thread pool ([7a79fe07](https://github.com/mozilla-services/syncstorage-rs/commit/7a79fe0766790d2e799070046ffa7aa21e06cbd5), closes [#745](https://github.com/mozilla-services/syncstorage-rs/issues/745))

#### Chore

*   Update vendored SDK to use protobuf 2.16.2 (#747) ([39519bb8](https://github.com/mozilla-services/syncstorage-rs/commit/39519bb821fdf58ecf5842c6b818a58d53167135))



<a name="0.5.2"></a>
## 0.5.2 (2020-07-22)


#### Chore

*   Update Docker rust to 1.45 (#734) ([538abe4b](https://github.com/mozilla-services/syncstorage-rs/commit/538abe4badf7a17200cd1400ed85b0504dadc865))

#### Bug Fixes

*   avoid unneeded clones ([9c1c19f2](https://github.com/mozilla-services/syncstorage-rs/commit/9c1c19f262afb4057f1bc3473d77bc4c84592d35), closes [#736](https://github.com/mozilla-services/syncstorage-rs/issues/736))



<a name="0.5.1"></a>
## 0.5.1 (2020-07-21)


#### Features

*   make migrations play nice with existing databases. (#721) ([40b97fc3](https://github.com/mozilla-services/syncstorage-rs/commit/40b97fc331d088462e09cbc5949b961ef5b6d4a5), closes [#663](https://github.com/mozilla-services/syncstorage-rs/issues/663))

#### Bug Fixes

*   switch create_session to async (#733) ([7cd04bc9](https://github.com/mozilla-services/syncstorage-rs/commit/7cd04bc9b4245bfb2ffca5e09de99cf3dd5753a8), closes [#731](https://github.com/mozilla-services/syncstorage-rs/issues/731))



<a name="0.5.0"></a>
## 0.5.0 (2020-07-16)


#### Features

*   option to limit purgettl to range of fxa_uids ([695722a9](https://github.com/mozilla-services/syncstorage-rs/commit/695722a9b5286eab62b7f541a3479da5f2dd0a07), closes [#713](https://github.com/mozilla-services/syncstorage-rs/issues/713))
*   limit purge ttl to prior midnight (#708) ([198eb816](https://github.com/mozilla-services/syncstorage-rs/commit/198eb816bc4a090d987aa933b492ec187de1e8e8), closes [#707](https://github.com/mozilla-services/syncstorage-rs/issues/707))
*   add conditions, args to purge_ttl script (#668) ([2a14eb29](https://github.com/mozilla-services/syncstorage-rs/commit/2a14eb2973997e2637ff0894e593642ba9a729f3))

#### Refactor

*   clear new clippy warnings ([d918550a](https://github.com/mozilla-services/syncstorage-rs/commit/d918550a8cf5b72631d79fc2232050418dd101ec))

#### Bug Fixes

*   remove report_error from the transaction handler ([f0e4c62e](https://github.com/mozilla-services/syncstorage-rs/commit/f0e4c62e3cff366edc9fc798cbe7c94377cc4a8a), closes [#723](https://github.com/mozilla-services/syncstorage-rs/issues/723))
*   Replace batch index to resolve hot table problem (#720) ([c3ca80e6](https://github.com/mozilla-services/syncstorage-rs/commit/c3ca80e66e4084ebc9b6c6efd41dff361b466fb8), closes [#719](https://github.com/mozilla-services/syncstorage-rs/issues/719))
*   don't call begin twice for mysql's delete_all (#673) ([c93db759](https://github.com/mozilla-services/syncstorage-rs/commit/c93db75976eaaf262c6c972566e80cfc3809e810), closes [#639](https://github.com/mozilla-services/syncstorage-rs/issues/639), [#441](https://github.com/mozilla-services/syncstorage-rs/issues/441))



<a name="0.4.2"></a>
## 0.4.2 (2020-06-24)


#### Bug Fixes

*   don't reject firefox-ios dev builds ([f6f4a15e](https://github.com/mozilla-services/syncstorage-rs/commit/f6f4a15e3325f8dec18ee0e9b705a0eaf9ceafa8), closes [#683](https://github.com/mozilla-services/syncstorage-rs/issues/683))



<a name="0.4.1"></a>
## 0.4.1 (2020-06-11)


#### Bug Fixes

*   python image build needs stable docker git container ([93edc9f6](https://github.com/mozilla-services/syncstorage-rs/commit/93edc9f6d20300dc2355cf80850ebf6d67143f5c))



<a name="0.4.0"></a>
## 0.4.0 (2020-06-11)


#### Doc

*   update per sentry dev's rename to local (#628) ([456c857d](https://github.com/mozilla-services/syncstorage-rs/commit/456c857dc06192d671516bd17f474d59f51cae30))
*   Update instructions for running syncstorage-rs via Docker (#624) ([eb5fa003](https://github.com/mozilla-services/syncstorage-rs/commit/eb5fa003d183b81b146c12afd498e8bf3555f334))

#### Refactor

*   quiet clippy warnings ([b08a90f1](https://github.com/mozilla-services/syncstorage-rs/commit/b08a90f14ab8db1bf1c7dedfc35d59d0fb05d2ee))
*   Convert actix-web frontend *_bso calls to async await (#638) ([7203b8fb](https://github.com/mozilla-services/syncstorage-rs/commit/7203b8fb7f4ccaf6bfbd47cd5d21876ad641f653), closes [#543](https://github.com/mozilla-services/syncstorage-rs/issues/543))
*   convert actix-web front-end calls to async ([300f2852](https://github.com/mozilla-services/syncstorage-rs/commit/300f28524677c0d4200ed3f440ed48f06dd21899), closes [#541](https://github.com/mozilla-services/syncstorage-rs/issues/541), [#541](https://github.com/mozilla-services/syncstorage-rs/issues/541), [#541](https://github.com/mozilla-services/syncstorage-rs/issues/541), [#541](https://github.com/mozilla-services/syncstorage-rs/issues/541), [#541](https://github.com/mozilla-services/syncstorage-rs/issues/541))
*   use u64 instead of i64 for Offset.offset ([8f4f4407](https://github.com/mozilla-services/syncstorage-rs/commit/8f4f4407a6f03d8d3ee90539dff8b8e6836198a1), closes [#414](https://github.com/mozilla-services/syncstorage-rs/issues/414))

#### Features

*   build spanner python utils image (#661) ([2060601c](https://github.com/mozilla-services/syncstorage-rs/commit/2060601c483a09c50ae6c7809d5b658980ad3ad8))
*   log messages from middleware to sentry (#604) ([b6ced47a](https://github.com/mozilla-services/syncstorage-rs/commit/b6ced47a39c5932cfc25a37008f78ba03c3e2655), closes [#504](https://github.com/mozilla-services/syncstorage-rs/issues/504))
*   Allow for failure "replay" from failure file (#644) ([b0f1590f](https://github.com/mozilla-services/syncstorage-rs/commit/b0f1590f4a289163b7043d01af06968b082d02ac), closes [#642](https://github.com/mozilla-services/syncstorage-rs/issues/642))
*   Don't report Conflict errors to sentry (#623) ([b2d93418](https://github.com/mozilla-services/syncstorage-rs/commit/b2d9341824d3bb7b722e75a5aaaa2e4096007e20), closes [#614](https://github.com/mozilla-services/syncstorage-rs/issues/614))
*   add async to `delete_all` (#621) ([fdb366da](https://github.com/mozilla-services/syncstorage-rs/commit/fdb366da3837ad74ec7fe6e67ad02c62af790c85), closes [#615](https://github.com/mozilla-services/syncstorage-rs/issues/615))
*   emit Db pool metrics periodically (#605) ([c3d6946e](https://github.com/mozilla-services/syncstorage-rs/commit/c3d6946e041a321fc1e11783a02b767f8e73dbe1), closes [#406](https://github.com/mozilla-services/syncstorage-rs/issues/406))
*   add a --wipe_user mode ([16058f20](https://github.com/mozilla-services/syncstorage-rs/commit/16058f20a42564398f0f27a6adfc686ed774531d), closes [#596](https://github.com/mozilla-services/syncstorage-rs/issues/596))
*   latest ops requests ([edd0017d](https://github.com/mozilla-services/syncstorage-rs/commit/edd0017d2cf7cbade3225fc640d2df8377d55938))
*   Enable circleci remote docker layer caching, speeding up the ci builds. ([7d9d521a](https://github.com/mozilla-services/syncstorage-rs/commit/7d9d521ab675db112f9ec66fe54ba028543c8ead))

#### Bug Fixes

*   range check the header to avoid a panic (#664) ([b73e6ee2](https://github.com/mozilla-services/syncstorage-rs/commit/b73e6ee2c7bd0aef080fa04af1d60fb41946837f), closes [#647](https://github.com/mozilla-services/syncstorage-rs/issues/647))
*   Make `bso_num` in migrate_node less truthy (#637) ([fa96964f](https://github.com/mozilla-services/syncstorage-rs/commit/fa96964f0703c731ea11f4a05d31a81c16669ce7), closes [#636](https://github.com/mozilla-services/syncstorage-rs/issues/636))
*   don't classify AlreadyExists as a ConflictError (#635) ([07276667](https://github.com/mozilla-services/syncstorage-rs/commit/07276667a30bba299f1085a6c1b16465250894a2), closes [#633](https://github.com/mozilla-services/syncstorage-rs/issues/633))
*   Add retry and sleep to purge_ttl attempts (#620) ([38c3295b](https://github.com/mozilla-services/syncstorage-rs/commit/38c3295b16a3250d474ff2024e855675c803f1a4))
*   don't replace user_collections ([d6b2dc21](https://github.com/mozilla-services/syncstorage-rs/commit/d6b2dc2187de5a1877b79e2354aa5ac746ce823a))
*   convert user_id into bigint ([ab2606da](https://github.com/mozilla-services/syncstorage-rs/commit/ab2606daeb3f5a9def697b4f16ded02af4290329), closes [#470](https://github.com/mozilla-services/syncstorage-rs/issues/470))
*   convert user_id into bigint ([8b951137](https://github.com/mozilla-services/syncstorage-rs/commit/8b951137374218ac6d2ec23e5f2c975b45fc2105), closes [#470](https://github.com/mozilla-services/syncstorage-rs/issues/470))

#### Chore

*   default-run syncstorage ([24b600dd](https://github.com/mozilla-services/syncstorage-rs/commit/24b600dd45b883563d06a2545f8c305ad1331fd3))



<a name="0.3.4"></a>
## 0.3.4 (2020-05-13)


#### Bug Fixes

*   don't consider expiry during batch commit (#632) ([90ff7485](https://github.com/mozilla-services/syncstorage-rs/commit/90ff74858f10f5e52f1acd60a57f6a2ead46c891))



<a name="0.3.3"></a>
## 0.3.3 (2020-05-11)


#### Features

*   include a hostname tag w/ pool metrics (#627) ([f11c04b5](https://github.com/mozilla-services/syncstorage-rs/commit/f11c04b530ef738703d87b8ea9c882bbfe21df80), closes [#555](https://github.com/mozilla-services/syncstorage-rs/issues/555))



<a name="0.3.2"></a>
## 0.3.2 (2020-05-05)


#### Chore

*   cargo fmt/clippy ([c17682fa](https://github.com/mozilla-services/syncstorage-rs/commit/c17682fa464c89faea4cb2e384a6c8747834d2dc))

#### Features

*   emit Db pool metrics periodically (#605) ([1761f7c7](https://github.com/mozilla-services/syncstorage-rs/commit/1761f7c7f1ee40de0563ebca2a23d50b0995fcee), closes [#406](https://github.com/mozilla-services/syncstorage-rs/issues/406))



<a name="0.3.1"></a>
## 0.3.1 (2020-04-21)


#### Bug Fixes

*   restore delete_bso's handling of errors ([c11e7894](https://github.com/mozilla-services/syncstorage-rs/commit/c11e78948ef507b7eb74743a02df95f907ba9a08), closes [#599](https://github.com/mozilla-services/syncstorage-rs/issues/599))



<a name="0.3.0"></a>
## 0.3.0 (2020-04-09)


#### Bug Fixes

*   add build_essential package to Dockerfile. ([05b20eca](https://github.com/mozilla-services/syncstorage-rs/commit/05b20eca8be5f3b5322d92cd73bcd42ddcfde2e3), closes [#572](https://github.com/mozilla-services/syncstorage-rs/issues/572))
*   do not populate mysql CollectionCache with invalid values ([0741104e](https://github.com/mozilla-services/syncstorage-rs/commit/0741104ec8d516b5ebe25399e2baa805a5d207a5), closes [#239](https://github.com/mozilla-services/syncstorage-rs/issues/239))
*   correct the test version of post_bsos ([f9842af9](https://github.com/mozilla-services/syncstorage-rs/commit/f9842af9fc7cc931d40205f5a7668cc1e5828d6b), closes [#533](https://github.com/mozilla-services/syncstorage-rs/issues/533))
*   Reduce log release_max_levels ([17ff2867](https://github.com/mozilla-services/syncstorage-rs/commit/17ff2867442e7600f121976c04af32fc4eb7632a))
*   `cargo clippy` for rust 1.42 ([546d96ca](https://github.com/mozilla-services/syncstorage-rs/commit/546d96ca2885003e4d912a72bccf33f2f6fcb1f2))
*   Convert erect_tombstone to async/await ([442c4c05](https://github.com/mozilla-services/syncstorage-rs/commit/442c4c05a1939b70d9632ce2228e036ef8d7589c))
*   revert unsupported config change ([f4cfcab1](https://github.com/mozilla-services/syncstorage-rs/commit/f4cfcab1771870674ad49e409ec33a43838c842f))
*   adapt to async ([fceea69e](https://github.com/mozilla-services/syncstorage-rs/commit/fceea69e324b3d4d33b8d06eb614f1e944996a9b))
*   Fix #444 invalid offset code that was lost in the actix 2 upgrade due to a bad merge ([efbf6594](https://github.com/mozilla-services/syncstorage-rs/commit/efbf65948fc42e0f7f23cfd051814dba3b399ded))
*   Fix #459 db-tests on master ([0cd2b9db](https://github.com/mozilla-services/syncstorage-rs/commit/0cd2b9db969cdf515ae46f939bdaee5a3a1edd4d))
*   Fix #457 remaining blocking execute ([3ed7ae62](https://github.com/mozilla-services/syncstorage-rs/commit/3ed7ae62d8ad0ccb5f765a7b8b6397ce110d30ea))
*   convert migration state to smallint (#429) ([b980b438](https://github.com/mozilla-services/syncstorage-rs/commit/b980b43872d8adca1c08ed56920b1da2d74fb329), closes [#428](https://github.com/mozilla-services/syncstorage-rs/issues/428))

#### Features

*   reject firefox-ios < 20 w/ a 503 ([337275c3](https://github.com/mozilla-services/syncstorage-rs/commit/337275c349c9acaa4965a755ba38126fadd53f38), closes [#293](https://github.com/mozilla-services/syncstorage-rs/issues/293))
*   specify database in user_migration/fix_collections.sql to help running from automation ([cbe3452c](https://github.com/mozilla-services/syncstorage-rs/commit/cbe3452c9d7cc9d968e49b075c8110b65d63fc4e))
*   add `--user_percent` option ([08a646a3](https://github.com/mozilla-services/syncstorage-rs/commit/08a646a36e9d1eda589dd21586ad3b3e4fe41f15))
*   add an extra sanity check of the db url ([f58b3bc9](https://github.com/mozilla-services/syncstorage-rs/commit/f58b3bc9b7bd069fb17090ff8cb440f4126610b5))
*   Add `--abort` and `--user_range` flags ([a65123bc](https://github.com/mozilla-services/syncstorage-rs/commit/a65123bcf2756cf2c6212cb683918c2bd83d692e))
*   more user_migration stuff (#450) ([ecfca9fd](https://github.com/mozilla-services/syncstorage-rs/commit/ecfca9fdf5b040abfa34b0c60daf19e0136adabf))
*   separately metric batch update/insert ([33065a8f](https://github.com/mozilla-services/syncstorage-rs/commit/33065a8f78fa978b990df043c841f663f4682157), closes [#454](https://github.com/mozilla-services/syncstorage-rs/issues/454))

#### Refactor

*   Remove python dependency from the dockerfile. ([3cd80947](https://github.com/mozilla-services/syncstorage-rs/commit/3cd809474573588471611c0e13e640530cbc588e), closes [#567](https://github.com/mozilla-services/syncstorage-rs/issues/567))
*   rewrite purge_ttl in Rust ([5d6d7c1a](https://github.com/mozilla-services/syncstorage-rs/commit/5d6d7c1a8aef941134aae2ea24a8d3ed0c4a0c15))
*   Convert the rest of the spanner module to async await ([e2017bbc](https://github.com/mozilla-services/syncstorage-rs/commit/e2017bbc2aee60399da2e9b750b7ecce856c4559))
*   Fix #442 Use map_ok in handlers to simplify the code and improve error reporting. ([c50b4cca](https://github.com/mozilla-services/syncstorage-rs/commit/c50b4cca22dc1a6c83757c2c63d719f2753054bf))
*   Fix #453 Convert straggler functions to async await ([69d50d2a](https://github.com/mozilla-services/syncstorage-rs/commit/69d50d2a3cdcf8f2b50acdd20c61743c50c014bc))
*   Fix #435 Convert db batch calls to async await. ([a9eeddb1](https://github.com/mozilla-services/syncstorage-rs/commit/a9eeddb14cdd0ecfc084307d751970656e2f842b))
*   Fix #433 Convert database bso calls to async await ([9279782f](https://github.com/mozilla-services/syncstorage-rs/commit/9279782f607fa87577f49f86a6017515f7c5d2b0))
*   Fix #434 Convert db collectioon calls to async await. ([e0b1c1cd](https://github.com/mozilla-services/syncstorage-rs/commit/e0b1c1cd1d6cfa227554fe670486525b413aa4bf))

#### Test

*   move db-tests back into the main crate (#465) ([f6990853](https://github.com/mozilla-services/syncstorage-rs/commit/f699085363b28bd0ea5c71f6f4231fa1df068fc0), closes [#410](https://github.com/mozilla-services/syncstorage-rs/issues/410))

#### Doc

*   fix typos in README.md files Fixed typos in README.md files to improve readiblity. ([7da2154b](https://github.com/mozilla-services/syncstorage-rs/commit/7da2154bcc2bc7618bf414d60212c2c2d2cfac5a), closes [#529](https://github.com/mozilla-services/syncstorage-rs/issues/529))
*   fix URL rendering in README ([bcb0e2e2](https://github.com/mozilla-services/syncstorage-rs/commit/bcb0e2e212554160978f206970e0856508840eb2), closes [#496](https://github.com/mozilla-services/syncstorage-rs/issues/496))
*   add system dependencies to README ([f0183495](https://github.com/mozilla-services/syncstorage-rs/commit/f01834957e5ced9989969f28ff4c3e6f23b2bf29), closes [#255](https://github.com/mozilla-services/syncstorage-rs/issues/255))

#### Chore

*   remove unused dependencies ([382f342a](https://github.com/mozilla-services/syncstorage-rs/commit/382f342a4c95641e8de1c0700648c922a6abc095))
*   Update dependencies 2020-03 ([7825ead1](https://github.com/mozilla-services/syncstorage-rs/commit/7825ead15313c50fcb41d2a48c0f13245a5c6024), closes [#537](https://github.com/mozilla-services/syncstorage-rs/issues/537))
*   move `insert into` to the bottom of ddl ([0203261e](https://github.com/mozilla-services/syncstorage-rs/commit/0203261ea6967bf5bda7a6284e1c3fc5edcd1238), closes [#473](https://github.com/mozilla-services/syncstorage-rs/issues/473))
*   remove custom async_test implementation ([3cbc3a1c](https://github.com/mozilla-services/syncstorage-rs/commit/3cbc3a1cf1f0137c8d23c8592b5ac805151413e9), closes [#461](https://github.com/mozilla-services/syncstorage-rs/issues/461))
*   re-add gcp-grpc deps setup ([aa7495d9](https://github.com/mozilla-services/syncstorage-rs/commit/aa7495d9151950431c5f67a5c61e16bdf02efcae))
*   kill checkout-gcp-grpc ([625a1c9f](https://github.com/mozilla-services/syncstorage-rs/commit/625a1c9f8b3e6779352dd97d5bffeaaff5df45ee))
*   add minumum supported rust version ([9740221a](https://github.com/mozilla-services/syncstorage-rs/commit/9740221aea93f4872e6369522aa55f0a93c3742a))
*   add a badge for matrix ([cd23e152](https://github.com/mozilla-services/syncstorage-rs/commit/cd23e15288ba6f9295ab7d0083b21edbdaa464b6))
*   Update to actix-web 2.0. ([a79434a9](https://github.com/mozilla-services/syncstorage-rs/commit/a79434a9e721f639bdda339bc601dc152451a1bb))



<a name="0.2.9"></a>
## 0.2.9 (2020-04-02)


#### Features

*   revert the GET collection sort order (c95f2ff) ([81b1e3f3](https://github.com/mozilla-services/syncstorage-rs/commit/81b1e3f3d1efcb82c25393222282560b6d09e64e), closes [#559](https://github.com/mozilla-services/syncstorage-rs/issues/559))



<a name="0.2.8"></a>
## 0.2.8 (2020-03-26)


#### Bug Fixes

*   allow hostnames for STATSD_HOST ([9c784055](https://github.com/mozilla-services/syncstorage-rs/commit/9c784055e109b49c808520fd1b02514c60a8f0d2), closes [#548](https://github.com/mozilla-services/syncstorage-rs/issues/548))



<a name="0.2.7"></a>
## 0.2.7 (2020-03-24)


#### Chore

*   adapt googleapis-raw dep to 0.2 branch ([58f2051f](https://github.com/mozilla-services/syncstorage-rs/commit/58f2051f42aec49006a3127a5f35a3b58b8e3a2d))

#### Refactor

*   clippy ([acadfc80](https://github.com/mozilla-services/syncstorage-rs/commit/acadfc80fd96b2d2f50d97733bdf3fa421462074))
*   rewrite purge_ttl in Rust ([2d351956](https://github.com/mozilla-services/syncstorage-rs/commit/2d351956c2fc0c818e1089974e7a6c1528ab15a5))



<a name="0.2.5"></a>
## 0.2.5 (2020-03-11)


#### Bug Fixes

*   relax MAX_TTL to 9 digits ([9b5bda50](https://github.com/mozilla-services/syncstorage-rs/commit/9b5bda5092ffa8852a812ba4f406358b0e6b780a), closes [#480](https://github.com/mozilla-services/syncstorage-rs/issues/480))



<a name="0.2.4"></a>
## 0.2.4 (2020-03-10)


#### Bug Fixes

*   GETs with a limit and no sort never advance X-Weave-Next-Offset ([c95f2ff2](https://github.com/mozilla-services/syncstorage-rs/commit/c95f2ff21a5e3b428b2715018e7e782b22a2dfa8))



<a name="0.2.2"></a>
## 0.2.2 (2020-02-12)


#### Chore

*   revert temp. sentry tags for the mutation limit issue ([f213a79c](https://github.com/mozilla-services/syncstorage-rs/commit/f213a79ce6ceffdec37660fcb21b8dac77f902bd), closes [#389](https://github.com/mozilla-services/syncstorage-rs/issues/389))

#### Performance

*   Port get_bsos' pagination optimization ([9266f753](https://github.com/mozilla-services/syncstorage-rs/commit/9266f753cfdfc3203673eaf2fafb0899b2c76233))

#### Features

*   restrict release mode logging to ERROR (#427) ([9ab20845](https://github.com/mozilla-services/syncstorage-rs/commit/9ab208452cbec48e26e10420fabf7031d5238e3e), closes [#426](https://github.com/mozilla-services/syncstorage-rs/issues/426))
*   recategorize logging messages into appropriate states ([d8aeb3ee](https://github.com/mozilla-services/syncstorage-rs/commit/d8aeb3ee88086c15632475bbface2c727b5d305d), closes [#416](https://github.com/mozilla-services/syncstorage-rs/issues/416))
*   script to count total users in spanner ([13d2490d](https://github.com/mozilla-services/syncstorage-rs/commit/13d2490df47531b93875573ae1e9e60388643d67))
*   User migration scripts ([3500b9b9](https://github.com/mozilla-services/syncstorage-rs/commit/3500b9b9055e776f564129103f9dff4831392e54))

#### Refactor

*   kill unnecessary copies from protobuf Values ([0de96712](https://github.com/mozilla-services/syncstorage-rs/commit/0de96712f05253c6aa55da2ee0aa875093837d0d), closes [#422](https://github.com/mozilla-services/syncstorage-rs/issues/422))

#### Bug Fixes

*   filter out variable data from URI metric (#421) ([3986c451](https://github.com/mozilla-services/syncstorage-rs/commit/3986c451a7e096d6924478c22984becaf4d5f41d), closes [#420](https://github.com/mozilla-services/syncstorage-rs/issues/420))



<a name="0.2.1"></a>
## 0.2.1 (2020-01-11)


#### Features

*   add basic logging to stdout and statsd metrics for purge_ttl.py ([92a57e65](https://github.com/mozilla-services/syncstorage-rs/commit/92a57e653d2e831eb0c78505683bbef536d68c79))

#### Bug Fixes

*   Don't report `uri.path` to Metrics ([68f8dcce](https://github.com/mozilla-services/syncstorage-rs/commit/68f8dcce48d8cf284a659c7f9e6dd2bdaa28380d), closes [#408](https://github.com/mozilla-services/syncstorage-rs/issues/408))
*   Don't return empty strings in tags. ([13a881b8](https://github.com/mozilla-services/syncstorage-rs/commit/13a881b87f7131dc3674f471cd08d1ad91daecd7), closes [#404](https://github.com/mozilla-services/syncstorage-rs/issues/404))
*   Use HttpResponse::build(status) instead of HttpResponse::Ok().status(status) ([67113c7b](https://github.com/mozilla-services/syncstorage-rs/commit/67113c7bb79359c310f59b348ffa4e11fa16c78e), closes [#393](https://github.com/mozilla-services/syncstorage-rs/issues/393))



<a name="0.1.14"></a>
##  0.1.14 (2020-01-06)


#### Doc

*   add more detailed sentry testing info ([681f1014](https://github.com/mozilla-services/syncstorage-rs/commit/681f1014891b39aea26af4390153f95d9a3ec22a))

#### Features

*   break apart middleware.rs (#392) ([5b0fb643](https://github.com/mozilla-services/syncstorage-rs/commit/5b0fb643e662117266a711d01c883b26781d4c2d), closes [#391](https://github.com/mozilla-services/syncstorage-rs/issues/391))
*   route reads through the streaming sql api ([0e539d50](https://github.com/mozilla-services/syncstorage-rs/commit/0e539d50d4a1154f5cc880faf5daa2482a1373fe), closes [#205](https://github.com/mozilla-services/syncstorage-rs/issues/205))

#### Bug Fixes

*   add tag info to sentry error messages (#372) ([b834c54a](https://github.com/mozilla-services/syncstorage-rs/commit/b834c54af693e7bbdfd2ec7038390a6f18413117))
*   ignore the collection field in POSTS also ([e1a530ba](https://github.com/mozilla-services/syncstorage-rs/commit/e1a530ba779dcdd0cd74fbd0edf6022b7bd73caf), closes [#376](https://github.com/mozilla-services/syncstorage-rs/issues/376))

#### Chore

*   remove travis related docs/links ([7c169145](https://github.com/mozilla-services/syncstorage-rs/commit/7c169145dab2266cbdab2235065228abd4a7fc1f))
*   add python to docker image ([e1f48b48](https://github.com/mozilla-services/syncstorage-rs/commit/e1f48b48c8d876ba64c8d8e7dfbf7b7962662741))



<a name="0.1.8"></a>
### 0.1.8 (2019-12-03)


#### Doc

*   add descriptive comment ([84f25af5](https://github.com/mozilla-services/syncstorage-rs/commit/84f25af5e36c13f69d0c422c15783420051613a7))
*   adjust PR template, finish combining READMEs ([bbe744dd](https://github.com/mozilla-services/syncstorage-rs/commit/bbe744ddba933abac5e667e5374bc35b0b1832ee), closes [#344](https://github.com/mozilla-services/syncstorage-rs/issues/344))
*   combining setup instructions into one main doc ([a8ead778](https://github.com/mozilla-services/syncstorage-rs/commit/a8ead778b6d955b92d7d915dd72f0f78ad30bad7))

#### Bug Fixes

*   optimize batch commit mutations ([5dd3c651](https://github.com/mozilla-services/syncstorage-rs/commit/5dd3c65143e535a65bc99b2e22784c48d4b7cf25), closes [#318](https://github.com/mozilla-services/syncstorage-rs/issues/318))
*   remove redundant syncstorage metric root ([a2083477](https://github.com/mozilla-services/syncstorage-rs/commit/a2083477b9ebc95787cb51fea85ed1afc43f726c), closes [#346](https://github.com/mozilla-services/syncstorage-rs/issues/346))
*   specify the release name to sentry ([9cdfe7d7](https://github.com/mozilla-services/syncstorage-rs/commit/9cdfe7d7812281fb3c8d1c716ddd54be92edb8b4))

#### Chore

*   improve local logging ([d1a84219](https://github.com/mozilla-services/syncstorage-rs/commit/d1a842195849a78bcc7e8a048f65b069b85b808f), closes [#350](https://github.com/mozilla-services/syncstorage-rs/issues/350))
*   fix syntax and make one small formatting change to PR template ([11e47545](https://github.com/mozilla-services/syncstorage-rs/commit/11e4754558b217cbfa36dcb998e96e9a1057dfcc), closes [#344](https://github.com/mozilla-services/syncstorage-rs/issues/344))

#### Refactor

*   minor cleanup ([8dfb0d51](https://github.com/mozilla-services/syncstorage-rs/commit/8dfb0d5123310224ffe9b50701c3efbb938ebf61))



<a name="0.1.7"></a>
## 0.1.7 (2019-11-16)


#### Bug Fixes

*   correct max_total_records ([adca8d67](https://github.com/mozilla-services/syncstorage-rs/commit/adca8d67ccae1132381da5590f889adbef4654f5), closes [#333](https://github.com/mozilla-services/syncstorage-rs/issues/333))
*   bump the db worker thread pool size ([29358466](https://github.com/mozilla-services/syncstorage-rs/commit/29358466b637c680141e6e6a4b021e9ec8bef8ce), closes [#302](https://github.com/mozilla-services/syncstorage-rs/issues/302))
*   Metric timer should use millis ([58120d65](https://github.com/mozilla-services/syncstorage-rs/commit/58120d65003a38a592be784e6a4707a6c1e3fbf6), closes [#326](https://github.com/mozilla-services/syncstorage-rs/issues/326))

#### Chore

*   point to mozilla-services/mozilla-rust-sdk ([44186211](https://github.com/mozilla-services/syncstorage-rs/commit/441862119e59ea170359aa88e0dbe73f7b78565f), closes [#335](https://github.com/mozilla-services/syncstorage-rs/issues/335))
*   Update dockerfile to rust 1.39 ([f0451097](https://github.com/mozilla-services/syncstorage-rs/commit/f0451097bf00245929e71728f00cdaa4b9534355))

#### Features

*   Include user agent info in metric tags ([cbc7bf50](https://github.com/mozilla-services/syncstorage-rs/commit/cbc7bf503bf652751df80f33702ce2b9798c1c2b), closes [#329](https://github.com/mozilla-services/syncstorage-rs/issues/329))
*   Add debugging tools ([7d07a894](https://github.com/mozilla-services/syncstorage-rs/commit/7d07a8948fdeb8b273e8eae87aaef594a22fb9b7))
*   check spanner commit size, error out if "too large" ([7e5ddf3c](https://github.com/mozilla-services/syncstorage-rs/commit/7e5ddf3c3b48a328ba89deb9045d3570e5576ba1), closes [#320](https://github.com/mozilla-services/syncstorage-rs/issues/320))



<a name="0.1.6"></a>
## 0.1.6 (2019-11-06)


#### Features

*   rearrange the batch impl ([6db58786](https://github.com/mozilla-services/syncstorage-rs/commit/6db58786641fecb2f98243764cba0e924844a06a), closes [#299](https://github.com/mozilla-services/syncstorage-rs/issues/299))
*   get_bso_ids only loads the id column ([55ce9b03](https://github.com/mozilla-services/syncstorage-rs/commit/55ce9b03e4cf1021bf23cd32351895f632761be1), closes [#248](https://github.com/mozilla-services/syncstorage-rs/issues/248))
*   removed SpannerType enum ([c2a7ad28](https://github.com/mozilla-services/syncstorage-rs/commit/c2a7ad288086eaa68f53df23f70a09e09f5d8bea), closes [#261](https://github.com/mozilla-services/syncstorage-rs/issues/261))

#### Bug Fixes

*   recreate stale spanner sessions on checkout ([f822aec9](https://github.com/mozilla-services/syncstorage-rs/commit/f822aec9c7244032ff09d15db65921da4474891e), closes [#316](https://github.com/mozilla-services/syncstorage-rs/issues/316))
*   switch to slog's envlogger ([20b21bee](https://github.com/mozilla-services/syncstorage-rs/commit/20b21bee0b9cc447d889a6d057a641f9c24c6b27), closes [#310](https://github.com/mozilla-services/syncstorage-rs/issues/310))

#### Refactor

*   schema renames (again) ([beddaf60](https://github.com/mozilla-services/syncstorage-rs/commit/beddaf600f9f8e07d23f5991d1a92b00f2a9e912), closes [#313](https://github.com/mozilla-services/syncstorage-rs/issues/313))



<a name="0.1.5"></a>
## 0.1.5 (2019-10-24)


#### Features

*   workaround timeouts w/ larger db conn sizes ([3ea16124](https://github.com/mozilla-services/syncstorage-rs/commit/3ea161249b2b6ce9d940f363dfdd6bb2c9fffeb6), closes [#302](https://github.com/mozilla-services/syncstorage-rs/issues/302))
*   use actix_web header parsers ([a021171e](https://github.com/mozilla-services/syncstorage-rs/commit/a021171e2d8e9b7de9ca478cc14cfbaaaeda57fe), closes [#294](https://github.com/mozilla-services/syncstorage-rs/issues/294))
*   add spanner tools ([21fbdb46](https://github.com/mozilla-services/syncstorage-rs/commit/21fbdb46ae7878ae9ec154f8e796a0a1628ad181))
*   add tag support for metrics. ([f90cb2fe](https://github.com/mozilla-services/syncstorage-rs/commit/f90cb2fe681a0aaf64802cd89b2d22ca8d66459d), closes [#222](https://github.com/mozilla-services/syncstorage-rs/issues/222))
*   add tag support for metrics. ([cb8cff5a](https://github.com/mozilla-services/syncstorage-rs/commit/cb8cff5aa081816c167087a8b3bcb31e2d94712c), closes [#222](https://github.com/mozilla-services/syncstorage-rs/issues/222))

#### Bug Fixes

*   send logging output to MozLog ([b83429ee](https://github.com/mozilla-services/syncstorage-rs/commit/b83429ee9df327fa17e9f6aa8adf340a7335d70b), closes [#285](https://github.com/mozilla-services/syncstorage-rs/issues/285))



<a name="0.1.4"></a>
## 0.1.4 (2019-10-18)


#### Bug Fixes

*   switch sentry to its curl transport ([5cbd1974](https://github.com/mozilla-services/syncstorage-rs/commit/5cbd19744c13ef59f7fb0ba995231879c7a050d6), closes [#289](https://github.com/mozilla-services/syncstorage-rs/issues/289))
*   accept weighted content-type headers ([f3899695](https://github.com/mozilla-services/syncstorage-rs/commit/f389969517e60d41774ce71c4e7093a79c642ddd), closes [#287](https://github.com/mozilla-services/syncstorage-rs/issues/287))



<a name="0.1.2"></a>
## 0.1.2 (2019-10-12)


#### Bug Fixes

*   Be more permissive about content type headers ([53292fc9](https://github.com/mozilla-services/syncstorage-rs/commit/53292fc9c77394441ff8b6575943ad8e22883b75), closes [#264](https://github.com/mozilla-services/syncstorage-rs/issues/264))

#### Features

*   rewrite post/put_bsos w/ spanner mutations ([a25a6288](https://github.com/mozilla-services/syncstorage-rs/commit/a25a62881b12f29506511d4a5018167eac4fff7b), closes [#267](https://github.com/mozilla-services/syncstorage-rs/issues/267))



<a name="0.1.1"></a>
## 0.1.1 (2019-10-09)


#### Chore

*   fix cache key to include CIRCLE_TAG ([5d2434e1](https://github.com/mozilla-services/syncstorage-rs/commit/5d2434e1f593c6a92b90e359fbc917a4fae80403))
*   update language in response to PR suggestions ([4573736b](https://github.com/mozilla-services/syncstorage-rs/commit/4573736be9fc83408e9803cac3594de9824f2963))
*   adding initial PR template ([a4383ecc](https://github.com/mozilla-services/syncstorage-rs/commit/a4383ecc6e256b8fefd06ec0cd9574ed21191d5e))
*   remove last last_modified -> modified remnant ([b67a1bfc](https://github.com/mozilla-services/syncstorage-rs/commit/b67a1bfc7539e35f0411cf15c472d5ee2000cada))



<a name="0.1.0"></a>
## 0.1.0 (2019-10-04)


#### Features

*   Initial release
