#include "Tools/Chronometer.hpp"

#include "Brain/Behavior/DefenderActionProvider.hpp"


DefenderActionProvider::DefenderActionProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , defenderAction_(*this)
{
}

void DefenderActionProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");

  defenderAction_->valid = true;
}
