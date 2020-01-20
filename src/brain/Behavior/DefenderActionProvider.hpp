#pragma once

#include "Data/DefenderAction.hpp"

#include "Framework/Module.hpp"


class Brain;

class DefenderActionProvider : public Module<DefenderActionProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name = "DefenderActionProvider";
  /**
   * @brief DefenderActionProvider initializes members
   * @param manager a reference to brain
   */
  DefenderActionProvider(const ModuleManagerInterface& manager);
  /**
   * @brief cycle calculates the defender's action
   */
  void cycle();

private:
  Production<DefenderAction> defenderAction_;
};
