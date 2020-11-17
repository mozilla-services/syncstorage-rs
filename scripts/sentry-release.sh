sentry-cli releases set-commits --auto $VERSION
sentry-cli releases new -p syncstorage-prod $VERSION
sentry-cli releases set-commits --auto $VERSION
sentry-cli releases finalize $VERSION
