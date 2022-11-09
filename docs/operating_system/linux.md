# Linux

The Nao uses a Linux 5.4 real time kernel for intel processors ([linux-intel/preempt-rt](https://github.com/intel/linux-intel-lts/tree/5.4/preempt-rt)).

Most of the kernel configuration is done by the `meta-intel` layer for yocto.
Special modifications for the Nao robot are contained in the `meta-nao` layer and mainly consist of patches and kernel modules by aldebaran for chestboard communication.


