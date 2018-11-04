#pragma once

#include "Data/WalkingEngineWalkOutput.hpp"
#include "Tools/StateMachine/Option.hpp"
#include "WalkManState.hpp"

/**
 * @brief WalkOptionInterface a basic interface specifying the layout of all options used in the
 * WalkManager
 */
template <typename T>
class WalkOptionInterface : public Option
{
public:
  /**
   * @brief transition handling the transitions between the different internal option-states based
   * on the external wmState.
   * @param wmState a reference to the external state of the WalkManager
   */
  virtual void transition(const WalkManState& wmState) = 0;
  /**
   * @brief action performs actions based on the internal option-state. May call some
   * funcitons provided in the wmState.
   * @param wmState a reference to the external state of the WalkManager
   */
  virtual T action(WalkManState& wmState) = 0;
};
