import pathlib


# Path for the aliveness swapfile
ALIVENESS_PATH = pathlib.Path("/tmp/hulks/aliveness.yml")

# Path for aliveness lockfile (i.e. "AlivenessListener running?")
ALIVENESS_LOCK_PATH = pathlib.Path("/tmp/hulks/aliveness.lock")

# Timespan after which robots will be marked as "temporarily not alive"
ALIVENESS_TIMEOUT_ALIVE = 10  # in seconds

# Timespan after which robots will be considered offline
ALIVENESS_TIMEOUT_OFFLINE = 20  # in seconds

# Port for incoming aliveness messages
ALIVENESS_BROADCAST_PORT = 12440
