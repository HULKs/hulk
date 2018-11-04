#include <exception>
#include <stdexcept>

#include "Data/FieldDimensions.hpp"
#include "Data/PlayerConfiguration.hpp"

#ifdef ITTNOTIFY_FOUND
#include "Data/ImageData.hpp"
#endif


#include "print.h"

#include "Brain.hpp"

#include "Definitions/windows_definition_fix.hpp"


Brain::Brain(const std::vector<Sender*>& senders, const std::vector<Receiver*>& receivers, Debug& d, Configuration& c, RobotInterface& ri)
  : ModuleManagerInterface("Brain", ConfigurationType::HEAD, senders, receivers, d, c, ri)
{
  try
  {
    getDatabase().get<PlayerConfiguration>().init(configuration());
    getDatabase().produce(typeid(PlayerConfiguration));
    getDatabase().get<FieldDimensions>().init(configuration());
    getDatabase().produce(typeid(FieldDimensions));
  }
  catch (const std::exception& e)
  {
    print(e.what(), LogLevel::ERROR);
  }
  catch (...)
  {
    print("Unknown exception in Brain::init", LogLevel::ERROR);
  }

  if (!sortModules<Brain>())
  {
    throw std::runtime_error("There are circular dependencies between brain modules!");
  }

#ifdef ITTNOTIFY_FOUND
  brainTopDomain_ = __itt_domain_create("BrainTop");
  brainBottomDomain_ = __itt_domain_create("BrainBottom");
#endif
}

void Brain::cycle()
{
  getDatabase().receive();

#ifdef ITTNOTIFY_FOUND
  Camera currentType = robotInterface().getCurrentCameraType();
  __itt_domain* currentDomain = currentType == Camera::TOP ? brainTopDomain_ : brainBottomDomain_;
#endif

  for (auto& it : modules_)
  {
#ifdef ITTNOTIFY_FOUND
    __itt_task_begin(currentDomain, __itt_null, __itt_null, it.second);
    it.first->runCycle();
    __itt_task_end(currentDomain);
#else
    it->runCycle();
#endif
  }

  getDatabase().send();
}
