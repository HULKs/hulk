# Filtering

TODO: Elaborate

- FutureQueue/Filtering
    - Overview: Time diagram/plot
    - Motivation: Filters need to have monotonic updates
        - What needs a filter module to do in each cycle?
            - Roll-back temporary measurements from last cycle
            - Apply persistent measurements
            - Temporarily apply temporary measurements
    - FutureQueue (each Perception Cycler has one to communicate to Control)
        - Producer
            - announce
            - finalize
        - Consumer
            - consume
    - PersistentDatabases consumes from multiple FutureQueues and reorganizes data
        - persistent vs. temporary
    - PersistentInputs (Interface for the filter modules)
        - persistent vs. temporary
