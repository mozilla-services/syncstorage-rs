# Builds a Dataflow flex template image for the
# payload-link change-stream-to-Pub/Sub pipeline. The Java sources under
# src/ are the build input; upstream-customization.patch is reference
# documentation only (see README.md).
#
# Build:
#   docker build -t payload-link-dataflow:local \
#     tools/payload-link-dataflow
#
# Publish + register as a flex template (operator workflow):
#   docker tag payload-link-dataflow:local <REGISTRY>/payload-link-dataflow:<TAG>
#   docker push <REGISTRY>/payload-link-dataflow:<TAG>
#   gcloud dataflow flex-template build \
#     gs://<BUCKET>/templates/payload-link-dataflow.json \
#     --image <REGISTRY>/payload-link-dataflow:<TAG> \
#     --sdk-language JAVA \
#     --metadata-file tools/payload-link-dataflow/metadata.json

FROM maven:3.9-eclipse-temurin-17 AS build

WORKDIR /build
COPY pom.xml /build/pom.xml
# Warm the local Maven cache before copying sources so source edits do
# not re-fetch all dependencies on every rebuild.
RUN mvn -B -e dependency:go-offline

COPY src /build/src
RUN mvn -B -e package

# Pinned to a specific dated release, not ":latest", so the launcher's Beam
# version can't drift out from under the pipeline's pinned beam.version in
# pom.xml. Bump both together when upgrading Beam. Find newer tags at:
#   https://gcr.io/v2/dataflow-templates-base/java17-template-launcher-base/tags/list
FROM gcr.io/dataflow-templates-base/java17-template-launcher-base:20260803-rc00

ENV FLEX_TEMPLATE_JAVA_MAIN_CLASS=com.mozilla.sync.payloadlink.PayloadLinkChangesToPubSub
ENV FLEX_TEMPLATE_JAVA_CLASSPATH=/template/payload-link-dataflow.jar

COPY --from=build /build/target/payload-link-dataflow-bundled-*.jar /template/payload-link-dataflow.jar
