#pragma once

#include <Libs/json/json.h>
#include <Tools/Storage/UniValue/UniValue.h>
#include <boost/signals2.hpp>
#include <list>
#include <map>
#include <memory>
#include <stdexcept>

struct MountedConfiguration
{
  MountedConfiguration()
    : filename()
    , root()
    , changed(false)
  {
  }
  std::string filename;
  Uni::Value root;
  bool changed;
};

/**
 * @brief ConfigurationType indicates if a configuration file is body or head specific
 */
enum class ConfigurationType
{
  HEAD,
  BODY
};

/**
 * @brief ConfigurationCallback is a callback one can register that will be
 * called, when the corresponding value changes. The new value will be given
 * as a parameter.
 */
typedef boost::function<void(const Uni::Value& newValue)> ConfigurationCallback;

/**
 * @brief ConfigurationSignal is the boost signal that manages the ConfigurationCallbacks.
 */
typedef boost::signals2::signal<void(const Uni::Value& newValue)> ConfigurationSignal;
/**
 * @brief ConfigurationSignalMap maps keys to signals.
 */
typedef std::map<std::string, ConfigurationSignal*> ConfigurationSignalMap;

/**
 * @brief The Configuration class is a class that manages configuration
 * files and can merge multiple files into one configuration database.
 * @author Finn Poppinga
 */
class Configuration
{
private:
  typedef std::map<std::string, MountedConfiguration> mount_map_t;

  ConfigurationSignalMap map_;
  std::string hash(const std::string& mount, const std::string& key);

  mount_map_t mountPts_;
  const std::string basePath_;
  std::string naoHeadName_;
  std::string naoBodyName_;
  std::string locationName_;


  friend class TUHH;
  // Hide constructors.
  Configuration(const std::string& fileRoot);
  Configuration(Configuration& other) = delete;
  Configuration operator=(Configuration& other) = delete;

  /**
   * @brief mountFile mounts a file to a mountPoint. The first file that is to be mounted
   * should be the default file that is valid for all robots. If another file is mounted to the
   * same mount point, the configuration values are merged (using the other files values over the
   * default values). If you now save the file, "dirty" values should be saved into the last mounted
   * file location.
   * @param mount The name of the mount point
   * @param filename the file to load.
   * @return whether the file was found
   */
  bool mountFile(const std::string& mount, const std::string& filename);

public:
  /**
   * @brief ~Configuration frees configuration signals
   */
  ~Configuration();

  /**
   * @brief setNaoHeadName
   * @param name
   */
  void setNaoHeadName(std::string name)
  {
    naoHeadName_ = name;
  }

  /**
   * @brief setNaoBodyName
   * @param name
   */
  void setNaoBodyName(std::string name)
  {
    naoBodyName_ = name;
  }

  /**
   * @brief setLocationName
   * @param name
   */
  void setLocationName(std::string name)
  {
    locationName_ = name;
  }

  /**
   * @brief mount mounts a file relative to a base directory both default and nao specific
   * @param mount the name of the mount point
   * @param name the last part of the file name
   * @param type determines whether the file is head or body specific
   */
  void mount(const std::string& mount, const std::string& name, ConfigurationType type);

  /**
   * @brief checks if the property is available
   * @param mount the mount point identifier to get the key from
   * @param key the key of the configuration value
   * @return if the configuration value is existant
   */
  bool hasProperty(const std::string& mount, const std::string& key);

  /**
   * @brief get gets a value from the configuration database.
   * @param mount the mount point identifier to get the key from
   * @param key the key of the configuration value
   * @return the configuration value in the mount point mount with the key key.
   */
  Uni::Value& get(const std::string& mount, const std::string& key);

  /**
   * @brief get returns all values from the specified mountpoint m.
   * @param mount the mount point identifier
   * @return the values from the specified mountpoint m.
   */
  Uni::Value& get(const std::string& mount);

  /**
   * @brief set sets an value in the configuration that is mounted under the mountPoint mount. If
   * there is no such value yet, it will be created.
   * @param mount the mount point identifier to set/create the key in
   * @param key the key of the configuration value
   * @param value the value to set the chosen value to.s
   */
  void set(const std::string& mount, const std::string& key, const Uni::Value& value);

  /**
   * @brief save will save all changed configuration values to the respective last-mounted configuration file.
   */
  void save();

  /**
   * @brief getMountPoints
   * @return map of string names of mounted mountpoints to string name of the matching configuration file
   */
  std::map<std::string, std::string> getMountPoints();

  /**
   * @brief getKeyList
   * @return get string list of all registered keys under a specified mount point
   */
  std::list<std::string> getKeyList(std::string mountPoint);

  /**
   * @brief registerCallback
   * @param key
   * @param callback
   * @return
   */
  boost::signals2::connection registerCallback(const std::string& mount, const std::string& key, ConfigurationCallback callback);
};

class ConfigurationException : public std::runtime_error
{
public:
  enum ErrorType
  {
    INVALID_JSON_FILE,
    INVALID_KEY,
    FILE_NOT_FOUND,
    MOUNT_POINT_NOT_EXISTING,
    KEY_NOT_EXISTING,
    ERROR_WHILE_SAVING,
    ERROR_UNKNOWN
  };

  ConfigurationException(std::string s, ErrorType e)
    : std::runtime_error(s)
    , e_type_(e)
  {
  }
  ErrorType getErrorType()
  {
    return e_type_;
  }

private:
  ErrorType e_type_;
};
