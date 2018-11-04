#include "Tools/Chronometer.hpp"

#include "GoalDetection.hpp"

GoalDetection::GoalDetection(const ModuleManagerInterface& manager)
  : Module(manager)
  , goalData_(*this)
{
}

void GoalDetection::cycle()
{
  // Chronometer time(debug(), mount_ + ".cycle_time");
  // blank module
  goalData_->valid = false;
}
