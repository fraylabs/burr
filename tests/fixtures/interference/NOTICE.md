# Gearmotor contact regression

`gearmotor-contact.step` is a source-preserving reduction of
`Yellow_gearmotor_L.step`, by **hasecilu**, from the **FreeCAD-library**.
It retains the Metal and Yellow parts and their original placements; the other
parts were removed without re-exporting the geometry.

Source: https://github.com/FreeCAD/FreeCAD-library/blob/544a254e090eaf7bfbb6a9b69e249dc0d3d29d67/Electronics%20Parts/Motors/DC%20motor/Yellow_gearmotor/L_shape/Yellow_gearmotor_L.step

Licensed under Creative Commons Attribution 3.0 (CC BY 3.0):
https://creativecommons.org/licenses/by/3.0/

The two valid source solids have exactly zero OCCT Common volume and a minimum
distance of 4.5999648534689186e-11 mm. This fixture protects contact from being
reported as positive-volume interference.

The positive regression translates Metal by 0.5 mm along world X while keeping
Yellow fixed. OCCT Common then measures 82.5 mm³, and Burr must still report
that pair as interference. Together the two tests protect both contact handling
and detection of a real overlap in the same geometry.
