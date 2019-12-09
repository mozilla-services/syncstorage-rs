<a name="0.1.10"></a>
## 0.1.10 (2019-12-09)


#### Bug Fixes

*   remove duplicate header key from example script ([fea3b532](https://github.com/mozilla-services/syncstorage-rs/commit/fea3b532826b628559f8b31093923de05356b13a))
*   allow "collection" in BSOs (although we ignore it) ([013eaafd](https://github.com/mozilla-services/syncstorage-rs/commit/013eaafd35d3955de655b500d6d5e38c17e20458))
*   oops, properly interleave this index ([658ba6af](https://github.com/mozilla-services/syncstorage-rs/commit/658ba6af831b4e5c424ed6273e37d5ce87660c30), closes [#318](https://github.com/mozilla-services/syncstorage-rs/issues/318))

#### Chore

*   adjust perms on sample file ([e1b25603](https://github.com/mozilla-services/syncstorage-rs/commit/e1b2560345ee99bff30fadcdcb8433f88065a5f3))
*   adjust sample script so it works and adding docs to link to it ([80b0c591](https://github.com/mozilla-services/syncstorage-rs/commit/80b0c591f321bbfc5073f0de833ddc03d7819a20))



<a name="0.1.9"></a>
## 0.1.9 (2019-12-04)


#### Features

*   Handle empty bso body on /meta/global ([af7bf6d0](https://github.com/mozilla-services/syncstorage-rs/commit/af7bf6d05e7bb9e3745bab73706984e1f336bd55))

#### Bug Fixes

*   Don't run mysql tests when testing spanner. ([bdd4cbe5](https://github.com/mozilla-services/syncstorage-rs/commit/bdd4cbe538b22b4a5d91f5572783931efa9eceda), closes [#360](https://github.com/mozilla-services/syncstorage-rs/issues/360))



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



