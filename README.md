# DXF Welder

This project is inspired by https://github.com/FormerLurker/ArcWelderPlugin.

It joins many line segments into arcs or circles with a DXF, which is very helpful for applications like OpenSCAD which can only output line segments. This avoids CNC laser/mill jitters from overprecisely following the jagged line segments, resulting in jagged edges and poor tolerance.

## Running

Use `$ cargo run <infile.dxf> <outfile.dxf>`.