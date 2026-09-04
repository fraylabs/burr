# Optional design providers

Load this reference only when a Burr task needs to create or modify source
geometry. Existing-file browsing and checks do not need a provider.

## Provider contract

A provider is an independently maintained skill or tool that owns one source
domain. Burr coordinates it but does not absorb its domain instructions.

For each provider task:

1. Identify the authoritative source and requested output.
2. Use an already available provider that matches that source domain.
3. Let the provider perform its own domain validation.
4. Export STEP when assembly structure or mechanical review matters; use STL or
   GLB when visual inspection alone is sufficient.
5. Keep outputs inside the user's project and record which source produced
   them.
6. Return to Burr to browse, refresh, and run supported checks.

If the needed provider is unavailable, report the missing capability and ask
before installing anything. Do not silently substitute a different source
system, fetch unreviewed instructions at runtime, or edit an exported model in
place.

## Known upstream providers

### Mechanical CAD

Use the installed `cad` skill from
[earthtojake/text-to-cad](https://github.com/earthtojake/text-to-cad) when it is
available and appropriate. That provider owns parametric or other editable CAD
source and its STEP/STL/GLB exports. Burr owns the subsequent viewing and
supported geometry checks.

Typical route:

```text
physical part request -> CAD source -> STEP -> Burr
```

### KiCad electronics

Use the applicable installed skills from
[American-Embedded/KiStack](https://github.com/American-Embedded/kistack) for
KiCad electronics work. KiStack owns schematic and PCB decisions, ERC/DRC,
manufacturing outputs, and populated STEP export. Burr owns only the
mechanical viewing or external assembly-interference result.

Typical route:

```text
electronics request -> KiCad project -> populated STEP -> Burr
```

Do not load KiStack for a purely mechanical part.

## Mixed systems

For a system containing mechanical and electronic sources, keep each source
independent and combine only their exported geometry in the owning assembly.
Preserve useful component or subsystem names so Burr findings identify the
responsible source.

For example, an enclosure and board remain:

```text
CAD enclosure source ------> enclosure STEP --\
                                              +--> assembly STEP --> Burr
KiCad electronics source --> populated STEP --/
```

A downstream Burr pass supports only Burr's tested geometry pairs. It does not
replace the CAD provider's design validation or KiStack's electrical checks.
