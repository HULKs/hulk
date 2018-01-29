/**
 * @file DcmConnectorAL.h
 * @brief Class providing the framework towards the AL side.
 * @author <a href="mailto:stefan.kaufmann@tu-harburg.de">Stefan Kaufmann</a>
 * @author <a href="mailto:nicolas.riebesel@tuhh.de">Nicolas Riebesel</a>
 * @author <a href="mailto:oliver.tretau@tuhh.de">Oliver Tretau</a>
 *
 * The DcmConnectorAL class inherits some brokers and proxies mendatory for the
 * connection to the DCM module.
 */

#ifndef DCMCONNECTORAL_H
#define DCMCONNECTORAL_H

#include <vector>
#include <string>

#include <alproxies/dcmproxy.h>
#include <alproxies/almemoryproxy.h>
#include <alcommon/alproxy.h>
#include <alcommon/albroker.h>
#include <boost/signals/connection.hpp>
#include <boost/signal.hpp>

/**
 * @class DcmConnectorAL
 * @brief Connector to DCM module
 * @author <a href="mailto:stefan.kaufmann@tu-harburg.de">Stefan Kaufmann</a>
 * @author <a href="mailto:nicolas.riebesel@tuhh.de">Nicolas Riebesel</a>
 * @author <a href="mailto:oliver.tretau@tuhh.de">Oliver Tretau</a>
 *
 * This class realizes the connection to the DCM module, if it is compiled for
 * the real robot.
 */
class DcmConnectorAL
{
public:

  /**
   * @brief Initializes Proxies and creates useful aliases
   * @param parent The parent Broker
   */
  static void init(const boost::shared_ptr<AL::ALBroker>& parent);

  /**
   * @brief Returning the brocker
   * @return The broker
   */
  static boost::shared_ptr< AL::ALBroker > getBroker()
  {
    return parentBroker;
  }

  /**
   * @brief Creation of an alias
   * @param alias A vector containing as first element the name of the alias.
   *        The following elements are a list of devices which shall be part of
   *        the alias
   */
  static void createAlias(const std::vector<std::string>& alias);

  /**
   * @brief Get the pointer to data in ALMemory
   * @param key The key for the variable from which the pointer shall be
   *        returned.
   * @return The pointer
   */
  static float* getDataPtr(const char* key);

  /**
   * @brief Get the data in ALMemory
   * @param key The key for the variable which shall be returned.
   * @return A string containing the value
   */
  static std::string getDataString(const char* key);

  /**
   * @brief Get the DCM time
   * @return The time in ms
   */
  static int getTime();

  /**
   * @brief Bind a method to the signal which is sent just before DCM will run
   * @param subscriber The method which shall be bound
   * @return The connection
   */
  static boost::signals::connection bindPre(
      const boost::signal<void ()>::slot_function_type& subscriber);

  /**
   * @brief Bind a method to the signal which is sent right after DCM ran
   * @param subscriber The method which shall be bound
   * @return The connection
   */
  static boost::signals::connection bindPost(
      const boost::signal<void ()>::slot_function_type& subscriber);

private:

  // connection to AL Modules
  static boost::shared_ptr<AL::DCMProxy> dcmProxy;
  static boost::shared_ptr<AL::ALBroker> parentBroker;
  static boost::shared_ptr<AL::ALMemoryProxy> memProxy;

};

#endif // DCMCONNECTORAL_H
