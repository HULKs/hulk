# Building your own

## Bill of materials

The following materials are required for building the charging box. Names,
specifications and quantities preserve the original bill of materials. Supplier
stock and product URLs change; use these descriptions to search current catalogs
and check selected parts against the design. Unverified product links are omitted.

| Item | Amount |
| --- | --- |
| 16 port switch (TP-Link TL-SG116E) | 1 |
| Aluminium suit case (PeakTech 7270) | 1 |
| Switching power supply 36V 28A (Mean Well UHP-1000-36) | 1 |
| Inrush current limiter 16A (ICL-16R) | 1 |
| RCBO 30mA/16A | 1 |
| DIN rail | 1 |
| C20 mains inlet with switch | 1 |
| C19 mains cable | 1 |
| Insulated wire 1.0mm² red 50m | 1 |
| Insulated wire 1.0mm² black 50m | 1 |
| Ferrule 1.0mm² (100 pack) | 1 |
| Ferrule 1.5mm² (100 pack) | 1 |
| Fan 40mm × 40mm × 10mm | 4 |
| Cable lug M3 | 3 |
| Cable lug M4 | 6 |
| Blade terminal socket 6.35mm | 3 |
| Slow-blow fuse 2.5A 5×20mm | 20 |
| Fuse holder 2.5A 5×20mm | 40 |
| 6×1 socket header | 40 |
| 50×1 pin header | 2 |
| Schottky diode 40V 3A (1N5822) | 20 |
| LAN cable 2m | 14 |
| Barrel jack 5.5mm/2.1mm | 18 |
| XL4015 CC/CV module | 20 |
| Cable sleeve 10mm × 2m | 18 |
| XT60 mounting connector pair | 18 |
| M3 threaded insert (100 pack) | 1 |
| M4 threaded insert (50 pack) | 1 |
| M4 short threaded insert (50 pack) | 1 |
| M3×16 screws (100 pack) | 1 |
| M3 washer (100 pack) | 1 |
| M3 nuts (100 pack) | 1 |
| M4×16 screws | 4 |
| M4 washer | 4 |
| M4×20 countersunk screws | 16 |

Furthermore, a small amount of appropriately colored 1.5mm² wire is required for the mains voltage wiring.

## 3D printed parts and laser cut covers

We designed most of the charging box in Fusion360. Source files are in
`tools/charging-box/cad`, with exported parts in `tools/charging-box/cad/exports`.

Many components of the charging box are 3D printed. We printed everything except
the mains inlet cover in PETG; the mains inlet cover was printed using PC Blend.
The following parts from the `exports` subfolder have to be 3D printed:

| File | Amount | Description |
| --- | --- | --- |
| `AC-cover-holder.stl` | 2 | Holders for the AC cover plate (left top and bottom corners) |
| `AC-DC-cover-holder.stl` | 2 | Holders for both the AC and DC cover plates; print the second one mirrored |
| `DC-cover-holder.stl` | 2 | Holders for the DC cover plate (right top and bottom corners) |
| `DC-cover-handle.stl` | 2 | Handles on the left side of the DC cover for lifting it up |
| `fan-cover.stl` | 4 | Mesh covers for the fan inlets and outlets |
| `fan-holder.stl` | 4 | Holders for the fan, also used at the inlet without fans |
| `side-plate-1.stl` | 1 | Part 1 of the side plate with the charging and Ethernet ports |
| `side-plate-2.stl` | 1 | Part 2 of the side plate with the charging and Ethernet ports |
| `spacer.amf` | 20 | Spacers between holder PCBs and bottom plate |
| `rain-cover.stl` | 4 (optional) | Optional rain covers for the fan inlets and outlets |
| `rain-cover-hulks.stl` | 4 (optional) | Alternative rain covers with an embedded HULKs logo |

The cover plates (`AC-cover.dxf` and `DC-cover.dxf`) for the AC and DC parts are laser cut.

## Printed Circuit Boards (PCBs)

We designed a custom PCB using KiCad to mount the CC/CV modules, fuses and diodes.
Source files are in `tools/charging-box/pcb`, along with
`charging-box-gerbers.zip` containing the exported fabrication files.

Each PCB mounts four modules, so order five PCBs in total.

!!! warning

    Make sure to insert the diodes in the correct orientation when assembling the PCBs!
