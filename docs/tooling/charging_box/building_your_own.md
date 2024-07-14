# Building your own

## Bill of materials

The following materials are required for building the charging box:

| Item                                                                                                                                   | Amount |
| -------------------------------------------------------------------------------------------------------------------------------------- | ------ |
| [16 port switch](https://www.reichelt.de/switch-16-port-gigabit-ethernet-tplink-tl-sg116e-p240948.html)                                | 1      |
| [Aluminium suit case](https://www.reichelt.de/koffer-fuer-messgeraete-350-x-120-x-500-mm-peaktech-7270-p141222.html)                   | 1      |
| [Switching power supply 36V 28A](https://www.reichelt.de/schaltnetzteil-geschlossen-1000-w-36-v-28-a-mw-uhp-1000-36-p306677.html)      | 1      |
| [Inrush current limiter 16A](https://www.reichelt.de/einschaltstrombegrenzer-hutschiene-16-a-icl-16r-p306700.html)                     | 1      |
| [RCBO 30mA/16A](https://www.reichelt.de/fi-ls-schalter-char-b-unverzoegert-16-a-2-polig-lsfi-1356-6-p166211.html)                      | 1      |
| [DIN rail](https://www.reichelt.de/norm-tragschiene-fuer-verteilergehaeuse-hut-35x7-250-p62001.html)                                   | 1      |
| [C20 mains inlet with switch](https://www.reichelt.de/kaltgeraetestecker-mit-schalter-16-a-c20-kes-16-5-p53033.html)                   | 1      |
| [C19 Mains cable](https://www.reichelt.de/netzkabel-schutzkontakt-stecker-typ-e-f-90-gew-0-5m-km-sk0190-s005-p336494.html)             | 1      |
| [Insulated wire 1.0mm^2 red 50m](https://www.reichelt.de/schaltlitze-h05v-k-1-0-mm-50-m-rot-h05vk-1-0-50rt-p69493.html)                | 1      |
| [Insulated wire 1.0mm^2 black 50m](https://www.reichelt.de/schaltlitze-h05v-k-1-0-mm-50-m-schwarz-h05vk-1-0-50sw-p69494.html)          | 1      |
| [Ferrule 1.0mm^2 (100 pack)](https://www.reichelt.de/100er-pack-aderendhuelsen-isoliert-1-0mm--aehi-1-0-100-p24718.html)               | 1      |
| [Ferrule 1.5mm^2 (100 pack)](https://www.reichelt.de/100er-pack-aderendhuelsen-isoliert-1-5mm--aehi-1-5-100-p24719.html)               | 1      |
| [Fan 40mm x 40mm x 10mm](https://www.reichelt.de/axialluefter-40x40x10mm-12v-13-9m-h-28-2dba-sun-ee40101s1-1-p260552.html)             | 4      |
| [Cable lug M3](https://www.reichelt.de/gabel-kerbschuhe-fuer-m3-rot-vt-gk-r-3-p231363.html)                                            | 3      |
| [Cable lug M4](https://www.reichelt.de/gabel-kerbschuhe-fuer-m4-rot-vt-gk-r-4-p231364.html)                                            | 6      |
| [Blade terminal socket 6.35mm](https://www.reichelt.de/flachsteckerhuelse-vollisoliert-breite-6-35mm-gelb-vt-ifsh-g-6-35-p231370.html) | 3      |
| [Slow-blow fuse 2.5A 5x20mm](https://www.reichelt.de/feinsicherung-5x20mm-mitteltraege-2-5a-mtr-2-5a-p13246.html)                      | 20     |
| [Fuse holder 2.5A 5x20mm](https://www.reichelt.de/sicherungshalter-5x20mm-max-6-3a-500v-pl-120000-p14679.html)                         | 40     |
| [6x1 socket header](https://www.reichelt.de/buchsenleisten-2-54-mm-1x06-gerade-mpe-094-1-006-p119915.html)                             | 40     |
| [50x1 pin header](https://www.reichelt.de/50pol-stiftleiste-gerade-rm-2-54-sl-1x50g-2-54-p19508.html)                                  | 2      |
| [Schottky diode 40V 3A](https://www.reichelt.de/schottkydiode-40-v-3-a-do-201ad-1n-5822-tsc-p216714.html)                              | 20     |
| [LAN cable 2m](https://www.reichelt.de/cat-6-flachkabel-ftp-schwarz-2m-value-21990972-p333024.html)                                    | 14     |
| [Barrel jack 5.5mm/2.1mm](https://www.reichelt.de/hohlstecker-knickschutz-aussen-5-5-mm-innen-2-1-mm-hs-21-13-p249894.html)            | 18     |
| [XL4015 CC/CV module](https://www.ebay.de/itm/176018098237)                                                                            | 20     |
| [Cable sleeve 10mm x 2m](https://www.ebay.de/itm/171827787141)                                                                         | 18     |
| [XT60 mounting connector pair](https://www.ebay.de/itm/175132601002)                                                                   | 18     |
| [M3 threaded insert (100 pack)](https://cnckitchen.store/products/gewindeeinsatz-threaded-insert-m3-standard-100-stk-pcs)              | 1      |
| [M4 threaded insert (50 pack)](https://cnckitchen.store/products/gewindeeinsatz-threaded-insert-m4-standard-50-stk-pcs)                | 1      |
| [M4 short threaded insert (50 pack)](https://cnckitchen.store/products/gewindeeinsatz-threaded-insert-m4-short-50-stk-pcs)             | 1      |
| M3x16 screws (100 pack)                                                                                                                | 1      |
| M3 washer (100 pack)                                                                                                                   | 1      |
| M3 nuts (100 pack)                                                                                                                     | 1      |
| M4x16 screws                                                                                                                           | 4      |
| M4 washer                                                                                                                              | 4      |
| M4x20 countersunk screws                                                                                                               | 16     |

Furthermore, a small amount of appropriately colored 1.5mm^2 wire is required for the mains voltage wiring.

## 3D printed parts and laser cut covers

We designed most of the charging box in Fusion360. The source files are located in the [`tools/charging-box/cad`](https://github.com/HULKs/hulk/tree/main/tools/charging-box/cad) folder in our main repository. The exported parts can be found under [`tools/charging-box/cad/exports`](https://github.com/HULKs/hulk/tree/main/tools/charging-box/cad/exports).

Many components of the charging box are 3D printed. We printed everything except the mains inlet cover in PETG, which was printed using PC Blend.
The following parts from the `exports` subfolder have to be 3D printed:

| File                     | Amount       | Description                                                                |
| ------------------------ | ------------ | -------------------------------------------------------------------------- |
| `AC-cover-holder.stl`    | 2            | Holders for the AC cover plate (left top and bottom corners)               |
| `AC-DC-cover-holder.stl` | 2            | Holders for both the AC and DC cover plates, print the second one mirrored |
| `DC-cover-holder.stl`    | 2            | Holders for the DC cover plate (right top and bottom corners)              |
| `DC-cover-handle.stl`    | 2            | Handles on the left side of the DC cover for lifting it up                 |
| `fan-cover.stl`          | 4            | Mesh covers for the fan inlets and outlets                                 |
| `fan-holder.stl`         | 4            | Holders for the fan, also used at the inlet without fans                   |
| `side-plate-1.stl`       | 1            | Part 1 of the side plate with the charging and ethernet ports              |
| `side-plate-2.stl`       | 1            | Part 2 of the side plate with the charging and ethernet ports              |
| `spacer.amf`             | 20           | Spacers between holder PCBs and bottom plate                               |
| `rain-cover.stl`         | 4 (optional) | Optional rain covers for the fan inlets and outlets                        |
| `rain-cover-hulks.stl`   | 4 (optional) | Alternative version of the rain covers with an embedded HULKs logo         |

The cover plates (`AC-cover.dxf` and `DC-cover.dxf`) for the AC and DC parts are laser cut.

## Printed Circuit Boards (PCBs)

We designed a custom PCB using KiCad to mount the CC/CV modules, fuses and diodes.
The source files are found at [`tools/charging-box/pcb`](https://github.com/HULKs/hulk/tree/main/tools/charging-box/pcb) in our main repository.
In the same folder you can also find `charging-box-gerbers.zip` containing the exported fabrication files.

Each PCB mounts four modules, you thus have to order five PCBs in total.

!!! warning

    Make sure to insert the diodes in the correct orientation when assembling the PCBs!
