#pragma once

#include <vector>

#include "Framework/Messaging.hpp"
#include "Framework/ModuleManagerInterface.hpp"

class Brain : public ModuleManagerInterface
{
public:
  /**
   * @brief Brain creates the modules
   * @param senders the list of senders for brain
   * @param receivers the list of receivers for brain
   * @param d a reference to the Debug instance
   * @param c a reference to the Configuration instance
   * @param ri a reference to the RobotInterface instance
   */
  Brain(const std::vector<Sender*>& senders, const std::vector<Receiver*>& receivers, Debug& d, Configuration& c, RobotInterface& ri);
  /**
   * @brief cycle executes all brain modules
   */
  void cycle();

#ifdef ITTNOTIFY_FOUND
  __itt_domain* brainTopDomain_;
  __itt_domain* brainBottomDomain_;
#endif
};
