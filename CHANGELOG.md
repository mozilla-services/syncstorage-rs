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



