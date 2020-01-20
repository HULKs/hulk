#!/usr/bin/env python3

import sys

from hulks import aliveness


def main(argv):
    arg_verbose = len(argv) > 1 and argv[1] == "--verbose"

    ar = aliveness.receivers.AlivenessReceiver(verbose=arg_verbose)

    try:
        with aliveness.locks.listen_lock("script"):
            ar.start_receiving()

            print("Listening for aliveness; press any key to quit",
                  file=sys.stderr)

            if not arg_verbose:
                print("(Run with --verbose to see aliveness messages)",
                      file=sys.stderr)

            # Wait for <any key>
            input()
    except aliveness.errors.LockError as e:
        # This case doesn't need a verbose stack trace...
        print(str(e), file=sys.stderr)
        exit(-1)


if __name__ == "__main__":
    main(sys.argv)
