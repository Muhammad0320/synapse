use iceoryx2::prelude::*;
use iceoryx2::port::publisher::Publisher;
use iceoryx2::port::subscriber::Subscriber;

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct AgentThought {

    pub agent_id: [u8; 16],
    pub transaction_id: u64,
    pub payload_size: u32,

}

// Explicitely tells the compiler that this struct is safe for zero-copy transmission. // It has no heap pointer, only flat primitives 
unsafe impl ZeroCopySend for AgentThought {}

pub  struct CortexDataPlane  {
  
    service_name: ServiceName

}

impl CortexDataPlane {

    pub fn new(topic:  &str) -> Self {

        let service_name = ServiceName::new(topic).expect("Invalid topic name");

        CortexDataPlane { service_name }
    }

    // Creates a publisher that writes zero-copy data
    pub fn create_publisher(&self) -> Result<Publisher<ipc::Service, AgentThought, ()>, Box<dyn std::error::Error>> {

        let node = NodeBuilder::new().create::<ipc::Service>()?; 

        let service = node.service_builder(&self.service_name)
                    .publish_subscribe::<AgentThought>()
                    .open_or_create()?;
                    

        let publisher = service.publisher_builder().create()?; 

        Ok(publisher)
    }

    pub fn create_subscriber(&self) -> Result<Subscriber<ipc::Service, AgentThought, ()>, Box<dyn std::error::Error>> {

        let node = NodeBuilder::new().create::<ipc::Service>()?;

        let service = node.service_builder(&self.service_name)
        .publish_subscribe::<AgentThought>()
        .open_or_create()?;

        let subscriber = service.subscriber_builder().create()?; 

        Ok(subscriber)
    }

} 