#include <sstream>
#include <iostream>

#include "DcmConnectorAL.h"
#include "DcmConverter.hpp"

// initialization of static variables
boost::shared_ptr<AL::DCMProxy> DcmConnectorAL::dcmProxy =
    boost::shared_ptr<AL::DCMProxy>();
boost::shared_ptr<AL::ALMemoryProxy> DcmConnectorAL::memProxy =
    boost::shared_ptr<AL::ALMemoryProxy>();
boost::shared_ptr<AL::ALBroker> DcmConnectorAL::parentBroker;

void DcmConnectorAL::init(const boost::shared_ptr<AL::ALBroker>& parent)
{
  // initialize dcmProxy
  dcmProxy = parent->getDcmProxy();

  // initialize parent Broker
  parentBroker = parent;

  // create memory Proxy
  memProxy = boost::shared_ptr<AL::ALMemoryProxy>(new AL::ALMemoryProxy(parentBroker));

}

void DcmConnectorAL::createAlias(const std::vector<std::string>& alias)
{
  std::cout << "\033[0;34m[SHM_INFO\t]\033[0m " << "Creating Alias set: " << alias.at(0);
  AL::ALValue al = DcmConverter::convertAlias(alias);
  dcmProxy->createAlias(al);
  std::cout << " ...done\n";
}

float* DcmConnectorAL::getDataPtr(const char *key)
{
  return (float*) memProxy->getDataPtr(key);
}

std::string DcmConnectorAL::getDataString(const char *key)
{
  return (std::string) memProxy->getData(key);
}

int DcmConnectorAL::getTime()
{
  return dcmProxy->getTime(0);
}

boost::signals::connection DcmConnectorAL::bindPre(
    const boost::signal<void ()>::slot_function_type& subscriber)
{
  return parentBroker->getProxy("DCM")->getModule()->atPreProcess(subscriber);
}

boost::signals::connection DcmConnectorAL::bindPost(
    const boost::signal<void ()>::slot_function_type& subscriber)
{
  return parentBroker->getProxy("DCM")->getModule()->atPostProcess(subscriber);
}



