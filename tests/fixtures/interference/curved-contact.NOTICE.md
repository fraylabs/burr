This synthetic fixture is authored for Burr and licensed under the repository's
MIT terms. It contains no third-party CAD geometry.

`curved-contact.generate.py` constructs an OCCT-valid ring (radii 10 and 14,
height 10) and a radius-10 cylinder with its seam rotated by 7.5 degrees. OCCT
confirms zero common volume. Their polygonal approximations can overlap, so a
mesh witness within the sampling margin must remain unresolved.
