#pragma once

#include "Data/GoalData.hpp"
#include "Framework/Module.hpp"

class Brain;

class GoalDetection : public Module<GoalDetection, Brain>
{
public:
  ModuleName name__{"GoalDetection"};
  GoalDetection(const ModuleManagerInterface& manager);
  void cycle();

private:
  Production<GoalData> goalData_;
};
