#pragma once

#include "DevilSmashStandardMessage.hpp"

#include <cstdlib>
#include <ctime>
#include <iostream>
#include <limits>
#include <chrono>
#include <cstdint>
#include <time.h>

namespace DevilSmash
{
  class DevilSmashStandardMessageTest
  {
  public:
    bool test();

  private:
    bool randomBool();

    uint32_t randomInt(uint32_t min = 0, uint32_t max = std::numeric_limits<uint16_t>::max());

    template <typename T>
    inline void checkEqual(const T& expected, const T& got, double delta = 0.f)
    {
      if (got > expected + delta || got < expected - delta)
      {
        std::cout << "Expected: " << expected << " got " << got << "\n";
        assert(false);
      }
    }
  };
}
