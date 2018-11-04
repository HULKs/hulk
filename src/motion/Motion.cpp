#include <stdexcept>

#include "Data/FieldDimensions.hpp"
#include "Framework/Module.hpp"

#include "print.hpp"

#include "Motion.hpp"


Motion::Motion(const std::vector<Sender*>& senders, const std::vector<Receiver*>& receivers, Debug& d, Configuration& c, RobotInterface& ri)
  : ModuleManagerInterface("Motion", ConfigurationType::BODY, senders, receivers, d, c, ri)
{
  getDatabase().get<FieldDimensions>().init(configuration());
  getDatabase().produce(typeid(FieldDimensions));

  if (!sortModules<Motion>())
  {
    throw std::runtime_error("There are circular dependencies between motion modules!");
  }

#ifdef ITTNOTIFY_FOUND
  motionDomain_ = __itt_domain_create("Motion");
#endif
}

void Motion::cycle()
{
  getDatabase().receive();

  for (auto& it : modules_)
  {
#ifdef ITTNOTIFY_FOUND
    __itt_task_begin(motionDomain_, __itt_null, __itt_null, it.second);
    it.first->runCycle();
    __itt_task_end(motionDomain_);
#else
    it->runCycle();
#endif
  }

  getDatabase().send();
}
