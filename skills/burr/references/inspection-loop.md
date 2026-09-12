# Burr inspection and repair loop

Load this reference when a model needs assembly-interference evidence or a
source repair based on Burr.

## Choose the input

- Use STEP for assembly structure and the interference check.
- Use STL or GLB for visual inspection only.
- Start Burr on the narrowest folder containing the intended models.
- Select the intended model explicitly before collecting evidence.
- Ignore generated caches and previews that are not source artifacts.

For a meaningful STEP assembly check, use distinct external components with
clear occurrence names. Burr cannot infer whether arbitrary names represent a
board, enclosure, bracket, motor, or fastener.

## Preserve subsystem boundaries

Internal contacts inside a purchased or already validated subsystem can be
intentional. When checking a subsystem against an enclosure or chassis,
preserve its internal geometry but represent the subsystem and each external
interface under test as distinct occurrences. Do not delete components or fuse
unrelated external geometry merely to force a pass.

## Evaluate and repair

1. Record the model path, Checks outcome, summary, component count, and checked
   pair count.
2. For a failure, record the component names and select the finding so Burr
   highlights both occurrences.
3. Use X-ray for hidden collisions and Solid for exterior occlusion.
4. Choose a camera view that makes the result understandable.
5. Locate the source system that owns the geometry or placement.
6. Repair the smallest responsible source feature or parameter.
7. Regenerate the same output path and let Burr refresh it.
8. Repeat the check and the same visual view.

Keep a negative artifact when it materially proves Burr detects the intended
defect. Name fail and pass models by the condition they demonstrate.

## Incomplete results

`incomplete` is a stop condition for a clean interference claim. Causes include
unsupported file structure, too few distinct components, or an open or
inconclusive component mesh after tessellation.

A detailed vendor STEP can still be unsuitable as a collision mesh. Retain it
for visual review when useful and introduce a documented, closed collision
envelope only for the dimensions under test. Never present that envelope as
the exact component.

## Reporting

For `pass`, report the number of components and checked pairs. For `fail`, also
report every relevant component pair and finding type. For `incomplete`, report
the reason and do not collapse it into pass or fail.

Always distinguish visual observations from computed interference. Burr does
not currently prove minimum clearance, tolerance stacks, motion envelopes,
flexible-body behavior, fit class, strength, or manufacturability.
