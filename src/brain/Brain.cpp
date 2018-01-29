#include <exception>
#include <stdexcept>

#include "Data/FieldDimensions.hpp"
#include "Data/PlayerConfiguration.hpp"

#include "print.h"

#include "Brain.hpp"

#include "Definitions/windows_definition_fix.hpp"


Brain::Brain(const std::vector<Sender*>& senders, const std::vector<Receiver*>& receivers,
    Debug& d, Configuration& c, RobotInterface& ri)
  : ModuleManagerInterface("Brain", ConfigurationType::HEAD, senders, receivers,
      d, c, ri)
{
  try
  {
    getDatabase().get<PlayerConfiguration>().init(configuration());
    getDatabase().get<FieldDimensions>().init(configuration());
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
}

void Brain::cycle()
{
  getDatabase().receive();

  for (auto& module : modules_)
  {
    module->runCycle();
  }

  getDatabase().send();
}
