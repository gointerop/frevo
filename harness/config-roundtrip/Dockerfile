# The HAPI side of the config-roundtrip oracle (design.md section 7).
#
# The probe is compiled against the jars and classes inside the pinned image itself
# (main.war!/WEB-INF/lib and WEB-INF/classes), not against Maven Central copies, and is
# then added to the unchanged pinned image at /app/extra-classes/, the last entry of its
# ENTRYPOINT's -Dloader.path. The image's ENTRYPOINT is kept: every case runs through
# HAPI's own PropertiesLauncher with HAPI's class path, packaged application*.yaml files
# and logback.xml. loader.main comes from /app/loader.properties, which PropertiesLauncher
# reads from its home directory (the working directory, /app) without turning it into a
# system property.
ARG HAPI_IMAGE=hapiproject/hapi:v8.12.0-2@sha256:0ff86aac4c7bd13a1ee226444a70263bb46ed1c28ef543c30a44f865b80ca04e
ARG JDK_IMAGE=eclipse-temurin:21-jdk@sha256:8ad118b146aa92da815b576441acd6b42b38236797ad0b6edc8c200b85bb0734

FROM ${HAPI_IMAGE} AS hapi

FROM ${JDK_IMAGE} AS build
COPY --from=hapi /app/main.war /w/main.war
WORKDIR /w
RUN jar xf main.war WEB-INF/classes WEB-INF/lib
COPY src/main/java /w/src
RUN javac --release 21 -proc:none -encoding UTF-8 -Xlint:-options \
      -cp "WEB-INF/classes:WEB-INF/lib/*" -d /out $(find src -name '*.java')

FROM ${HAPI_IMAGE}
COPY --from=build /out/ /app/extra-classes/
COPY loader.properties /app/loader.properties
