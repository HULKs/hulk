#pragma once

#include "Data/MotionActivation.hpp"
#include "Data/PoserOutput.hpp"
#include "Framework/Module.hpp"

class Motion;

/**
 * @brief The Poser class will move the robot to a given pose.
 * @author Finn Poppinga
 */
class Poser : public Module<Poser, Motion>
{
public:
  /// the name of this module
  ModuleName name = "Poser";
  /**
   * @brief Poser initializes members
   * @param manager a reference to motion
   */
  Poser(const ModuleManagerInterface& manager);
  /**
   * @brief cycle checks for commands and sets a pose if needed
   */
  void cycle();

private:
  /// a reference to the motion activation
  const Dependency<MotionActivation> motionActivation_;
  /// a reference to the poser output
  Production<PoserOutput> poserOutput_;
};
