#include "Configuration.h"
#include <fstream>
#include <iostream>

#include "Tools/Storage/UniValue/UniValue2Json.hpp"
#include "Tools/Time.hpp"
#include "print.h"

Configuration::Configuration(const std::string& fileRoot)
  : mountPts_()
  , basePath_(fileRoot + "configuration/")
  , naoHeadName_("default")
  , naoBodyName_("default")
  , locationName_("default")
{
}

Configuration::~Configuration()
{
  for (auto& callback : map_)
  {
    delete callback.second;
  }
}

void Configuration::mount(const std::string& mount, const std::string& name, ConfigurationType type)
{
  bool found = false;
  std::string headBodyDefaultPath =
      ((type == ConfigurationType::HEAD) ? "head/" : "body/") + std::string("default/");
  std::string headBodyPath =
      ((type == ConfigurationType::HEAD) ? ("head/" + naoHeadName_) : ("body/" + naoBodyName_)) +
      "/";
  // Try the most generic configuration first and the most specific configuration last since it will
  // overwrite the previous values.
  std::string path = basePath_ + "location/default/" + name;
  if (mountFile(mount, path))
  {
    found = true;
  }
  path = basePath_ + "location/default/" + headBodyDefaultPath + name;
  if (mountFile(mount, path))
  {
    found = true;
  }
  path = basePath_ + "location/default/" + headBodyPath + name;
  if (mountFile(mount, path))
  {
    found = true;
  }
  if (locationName_ != "default")
  {
    std::string locationPath = basePath_ + "location/" + locationName_ + "/";
    path = locationPath + name;
    if (mountFile(mount, path))
    {
      found = true;
    }
    path = locationPath + headBodyDefaultPath + name;
    if (mountFile(mount, path))
    {
      found = true;
    }
    path = locationPath + headBodyPath + name;
    if (mountFile(mount, path))
    {
      found = true;
    }
  }

  if (!found)
  {
    throw ConfigurationException("Configuration file '" + name +
                                     "' does not exist in any configuration directory.",
                                 ConfigurationException::FILE_NOT_FOUND);
  }
}

bool Configuration::mountFile(const std::string& mount, const std::string& filename)
{
  MountedConfiguration from;

  from.filename = filename;
  Json::Reader reader;

  std::ifstream stream(filename);
  if (!stream.is_open())
  {
    return false;
  }
  try
  {
    Json::Value tmp;
    reader.parse(stream, tmp);
    from.root = Uni::Converter::toUniValue(tmp);
  }
  catch (std::exception& exc)
  {
    throw ConfigurationException(exc.what(), ConfigurationException::ERROR_UNKNOWN);
  }

  if (from.root.type() != Uni::ValueType::OBJECT)
  {
    throw ConfigurationException(
        "Configuration files must contain a Json::objectValue as root node!",
        ConfigurationException::INVALID_JSON_FILE);
  }


  if (mountPts_.find(mount) != mountPts_.end())
  {
    MountedConfiguration& to = mountPts_[mount];
    to.filename = from.filename;

    for (auto it = from.root.objectBegin(); it != from.root.objectEnd(); ++it)
    {
      to.root[it->first] = Uni::Value(it->second);
    }
  }
  else
  {
    mountPts_[mount] = from;
  }


  Log(LogLevel::DEBUG) << "mounted " << filename << " to " << mount;
  return true;
}

bool Configuration::hasProperty(const std::string& mount, const std::string& key)
{
  if (mountPts_.count(mount) > 0)
  {
    return mountPts_[mount].root.hasProperty(key);
  }
  else
  {
    throw ConfigurationException("Mount Point " + mount + " does not exist!",
                                 ConfigurationException::MOUNT_POINT_NOT_EXISTING);
  }
}

Uni::Value& Configuration::get(const std::string& mount, const std::string& key)
{
  if (mountPts_.count(mount) > 0)
  {
    if (!mountPts_[mount].root.hasProperty(key))
    {
      throw ConfigurationException("Key " + key + " does not exist in mount point " + mount + "!",
                                   ConfigurationException::KEY_NOT_EXISTING);
    }
    return mountPts_[mount].root[key];
  }
  else
  {
    throw ConfigurationException("Mount Point " + mount + " does not exist!",
                                 ConfigurationException::MOUNT_POINT_NOT_EXISTING);
  }
}

Uni::Value& Configuration::get(const std::string& mount)
{
  if (mountPts_.count(mount) > 0)
  {
    return mountPts_[mount].root;
  }
  else
  {
    throw ConfigurationException("Mount Point " + mount + " does not exist!",
                                 ConfigurationException::MOUNT_POINT_NOT_EXISTING);
  }
}

void Configuration::set(const std::string& mount, const std::string& key, const Uni::Value& value)
{
  if (mountPts_.find(mount) != mountPts_.end())
  {
    // key denotes the key that was received, realKey is the top-level part of the key as it is used
    // in the map.
    std::string realKey;
    std::size_t currentPos = 0;
    Uni::Value* currentValue = &(mountPts_[mount].root);
    Uni::Value* completeValue = nullptr; // completeValue is the Uni::Value that is in the map
    while (currentPos < key.length())
    {
      std::size_t dotPos = key.find('.', currentPos);
      std::size_t bracketPos = key.find('[', currentPos);
      std::size_t nextPos = std::min(dotPos, bracketPos);
      std::string thisKey =
          key.substr(currentPos, (nextPos == std::string::npos ? nextPos : nextPos - currentPos));
      // At this point, it is required that currentValue points to an object.
      // If thisKey doesn't exist it will be created as a value of type NIL.
      // After accessing the new value with [] or at, the type will be fixed to OBJECT or ARRAY (see
      // Uni::Value).
      currentValue = &((*currentValue)[thisKey]);
      if (completeValue == nullptr)
      {
        realKey = thisKey;
        completeValue = currentValue;
      }
      if (nextPos == std::string::npos)
      {
        break;
      }
      if (bracketPos < dotPos)
      {
        std::size_t closingBracketPos = key.find(']', bracketPos);
        if (closingBracketPos == std::string::npos)
        {
          throw ConfigurationException("Key has no matching closing bracket.",
                                       ConfigurationException::INVALID_KEY);
        }
        std::string indexString = key.substr(bracketPos + 1, closingBracketPos - bracketPos - 1);
        // TODO: check that everything is valid between the brackets. But this is something we might
        // not care.
        Uni::Value::valuesList_t::size_type index = std::stol(indexString);
        if (index >= currentValue->size())
        {
          // This is not supported yet because it is a potential security hole.
          // One could send something like key[10000000] which would allocate very much memory.
          throw ConfigurationException(
              "Enlarging arrays via Configuration::set is not supported yet.",
              ConfigurationException::INVALID_KEY);
        }
        currentValue = &(currentValue->at(index));
        nextPos = closingBracketPos;
      }
      currentPos = nextPos + 1;
    }
    if (currentValue == nullptr || completeValue == nullptr)
    {
      throw std::runtime_error(
          "Something went wrong in Configuration::set. This is probably a bug in the code.");
    }
    *currentValue = value;
    mountPts_[mount].changed = true;

    std::string h = hash(mount, realKey);
    auto it = map_.find(h);
    if (it != map_.end())
    {
      (*it->second)(*completeValue);
    }
  }
  else
  {
    throw ConfigurationException("Mount Point does not exist!",
                                 ConfigurationException::MOUNT_POINT_NOT_EXISTING);
  }
}

void Configuration::save()
{
  try
  {
    Json::StyledWriter writer;

    for (auto it = mountPts_.begin(); it != mountPts_.end(); ++it)
    {
      if (it->second.changed)
      {
        std::ofstream file(it->second.filename);
        file << writer.write(Uni::Converter::toJson(it->second.root));
        file.close();
      }
    }
  }
  catch (std::exception& e)
  {
    throw ConfigurationException("Can not save Configuration file to disk. " +
                                     std::string(e.what()),
                                 ConfigurationException::ERROR_WHILE_SAVING);
  }
}

std::string Configuration::hash(const std::string& mount, const std::string& key)
{
  return (mount + "#" + key);
}

std::map<std::string, std::string> Configuration::getMountPoints()
{
  std::map<std::string, std::string> ret;

  for (auto it = mountPts_.begin(); it != mountPts_.end(); it++)
  {
    ret[it->first] = it->second.filename;
  }

  return ret;
}

std::list<std::string> Configuration::getKeyList(std::string mountPoint)
{
  std::list<std::string> ret;
  Uni::Value& root = mountPts_.at(mountPoint).root;

  for (auto it = root.objectBegin(); it != root.objectEnd(); it++)
  {
    ret.push_back(it->first);
  }

  return ret;
}

boost::signals2::connection Configuration::registerCallback(const std::string& mount,
                                                            const std::string& key,
                                                            ConfigurationCallback callback)
{
  std::string h = hash(mount, key);
  auto it = map_.find(h);

  if (it == map_.end())
  {
    map_[h] = new ConfigurationSignal();
    it = map_.find(h);
  }

  return it->second->connect(callback);
}
