import asyncio

def make_thread_target(coroutine):
    """make_thread_target takes a coroutine and
    returns a thread target with the coroutine
    executed inside an event loop that is
    explicitly closed when complete
    """

    def target():
        # logger.debug(__name__ +
        #              ": Executing " + str(coroutine) +
        #              " in a new event loop.")
        loop = asyncio.new_event_loop()
        loop.run_until_complete(coroutine)
        loop.close()

    return target
