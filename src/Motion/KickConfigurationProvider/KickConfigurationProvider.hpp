#pragma once

#include "Data/KickConfigurationData.hpp"
#include "Framework/Module.hpp"

class Motion;

/**
 * @brief KickConfigurationProvider provides some general information for Motion and Brain modules
 * to perform kicks. These include information for positioning at the ball as well as kick
 * trajectory information
 */
class KickConfigurationProvider : public Module<KickConfigurationProvider, Motion>
{
public:
  /// the name of this module
  ModuleName name__{"KickConfigurationProvider"};
  /**
   *@brief The constructor of this class
   */
  KickConfigurationProvider(const ModuleManagerInterface& manager);

  void cycle();

private:
  /**
   * @brief updateOutput updates the output of this module if new configuration is loaded
   */
  void updateOutput();

  const Parameter<KickConfiguration> forwardKick_;

  /// a simple in walk kick to the front (kicking with the left foot)
  const Parameter<InWalkKick> inWalkFrontKick_;
  /// an in walk kick turning (kick with left foot, turning right)
  const Parameter<InWalkKick> inWalkTurnKick_;

  /// the kick configurations available to other modules
  Production<KickConfigurationData> kickConfigurationData_;

  /// true if some configuration changed this cycle
  bool configurationChanged_;
};
