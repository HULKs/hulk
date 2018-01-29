#include <stdexcept>

#include "Data/FieldDimensions.hpp"
#include "Framework/Module.hpp"

#include "print.hpp"

#include "Motion.hpp"


Motion::Motion(const std::vector<Sender*>& senders, const std::vector<Receiver*>& receivers,
    Debug& d, Configuration& c, RobotInterface& ri)
  : ModuleManagerInterface("Motion", ConfigurationType::BODY, senders, receivers,
      d, c, ri)
{
  getDatabase().get<FieldDimensions>().init(configuration());

  if (!sortModules<Motion>()) {
    throw std::runtime_error("There are circular dependencies between motion modules!");
  }
}

void Motion::cycle()
{
  getDatabase().receive();

  for (auto& it : modules_) {
    it->runCycle();
  }

  getDatabase().send();
}
