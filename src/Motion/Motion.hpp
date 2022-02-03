#pragma once

#include "Framework/ModuleManagerInterface.hpp"


class Motion : public ModuleManagerInterface
{
public:
  /**
   * @brief Motion creates all motion modules
   * @param senders the list of senders for motion
   * @param receivers the list of receivers for motion
   * @param d a reference to the Debug instance
   * @param c a reference to the Configuration instance
   * @param ri a reference to the RobotInterface instance
   */
  Motion(const std::vector<Sender*>& senders, const std::vector<Receiver*>& receivers, Debug& d,
         Configuration& c, RobotInterface& ri);
  /**
   * @brief cycle runs all motion modules
   */
  void cycle();

#ifdef ITTNOTIFY_FOUND
  __itt_domain* motionDomain_;
#endif
};
