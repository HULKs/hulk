#include "Time.hpp"


// Need to initialize static member outside class
#if defined(NAOV6)
std::chrono::time_point<std::chrono::steady_clock> TimePoint::baseTime_ =
    std::chrono::steady_clock::now() - std::chrono::milliseconds(2000);
#elif defined(NAOV5) || defined(REPLAY)
// "it looks so nice they did it twice"
std::chrono::time_point<std::chrono::system_clock> TimePoint::baseTime_ =
    std::chrono::system_clock::now() - std::chrono::milliseconds(15000);
#else
uint32_t TimePoint::baseTime_ = 0;
#endif
