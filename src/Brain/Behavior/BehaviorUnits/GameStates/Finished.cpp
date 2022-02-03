#include "Brain/Behavior/Units.hpp"

ActionCommand finished(const DataSet& d)
{
  if (d.gameControllerState.gamePhase == GamePhase::PENALTYSHOOT)
  {
    return ActionCommand::penalized();
  }
  return ActionCommand::sitDown();
}
