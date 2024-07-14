# Overview

To easily charge and deploy our NAOs during events, we have built a custom charging box able to provide power and a network connection to up to 14 NAOs at the same time.
This page describes how the charging box works and how you can build your own.

## Motivation and Design Considerations

Our previous charging box was limited to charging 8 NAOs/batteries at the same time. With the switch from 5 to 7 players on the field and the requirement for replacement robots it was not sufficient anymore for charging all of our NAOs at the same time.
This charging box in comparison is able to charge up to 18 NAOs/batteries concurrently while remaining in the same suitcase form factor as the previous one.

Like the previous one, it additionally contains a builtin 16-port network switch for providing a wired network connection for up to 14 charging robots. The remaining 2 ports are reserved for the uplink connection and future internal use.

The NAOs are connected to the box using detachable cables consisting of a pair of Ethernet and power cables. The previous charging box used XLR connectors for detaching the power cables while the new one uses XT60 connectors instead due to their smaller size, robustness and high current rating.

### Compactly charging 18 NAOs/batteries

The charging box achieves its giant increase in charging capabilities compared to the previous one by separating the AC to DC conversion part from the actual charging circuitry. Consider the rough schematic of the previous charging box:

```
              ┌─────────────┐
230V AC ──┬───┤ NAO charger ├─── 24.8V @ 2A (CC/CV)
          │   └─────────────┘
          │
          │   ┌─────────────┐
          ├───┤ NAO charger ├─── 24.8V @ 2A (CC/CV)
          │   └─────────────┘
          │
        [...]
          │
          │   ┌─────────────┐
          └───┤ NAO charger ├─── 24.8V @ 2A (CC/CV)
              └─────────────┘
```

Internally, the previous charging box used the chargers provided with the NAOs and just combined the AC inputs. Instead, this charging box uses the following concept:

```
           ┌──────────────┐
           │ Switching    │  36V  ┌──────────────┐
230V AC ───┤ power supply ├───┬───┤ CC/CV module ├─── 24.8V @ 2A (CC/CV)
           │ (36V DC)     │   │   └──────────────┘
           └──────────────┘   │
                              │   ┌──────────────┐
                              ├───┤ CC/CV module ├─── 24.8V @ 2A (CC/CV)
                              │   └──────────────┘
                              │
                            [...]
                              │
                              │   ┌──────────────┐
                              └───┤ CC/CV module ├─── 24.8V @ 2A (CC/CV)
                                  └──────────────┘
```

In the first stage, the mains voltage is converted to 36V DC using a switching power supply. This stage is shared among all charging modules.

The actual charging itself is performed by dedicated modules in the second stage: Those constant current/constant voltage (CC/CV) modules just have to perform DC to DC conversion and current limiting. This allows them to be significantly smaller than a single NAO charger allowing for the 2.5x increase in charging capabilities of espresso compared to the previous charging box.

### Additional safety features

Since there is some custom mains voltage wiring, we want to ensure the charging box is as safe as possible even in case of a fault. For the mains voltage side, we thus includes a RCBO providing leakage current and overcurrent protection. All mains wiring is isolated and within a separate isolated compartment which is labelled accordingly.

To ensure the breaker is not accidentally tripped by the inrush current of the main power supply, an inrush current limiter is added in between the RCBO and the main power supply.

On the 36V DC side, each CC/CV module is connected via a 2.5A slow-blow fuse.
