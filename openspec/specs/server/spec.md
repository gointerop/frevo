# server Specification

## Purpose

HTTP surface, interceptor pipeline and capability wiring.

## Requirements

### Requirement: Capability statement

The server SHALL respond to `GET /fhir/metadata` with a CapabilityStatement
describing only the capabilities it actually implements.

#### Scenario: [server.metadata.001] metadata returns a CapabilityStatement

- **WHEN** a client issues `GET /fhir/metadata`
- **THEN** the response is `200 OK` with content type `application/fhir+json`
- **AND** the body is a `CapabilityStatement` with `status` `active` and
  `fhirVersion` `4.0.1`

#### Scenario: [server.metadata.002] no capability is advertised that is not implemented

- **WHEN** a build implements no resource-level interactions
- **THEN** `rest[0].resource` in the CapabilityStatement is empty
