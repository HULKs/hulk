#pragma once
#include "Behavior/Units.hpp"

ActionCommand searchForBall(const DataSet& d)
{
  return walkToPose(d, d.ballSearchPosition.pose, true, WalkMode::PATH, Velocity(), 5)
    .combineHead(lookAround(d));
}
