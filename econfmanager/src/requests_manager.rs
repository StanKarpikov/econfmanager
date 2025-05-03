use tonic::{transport::Server, Request, Response, Status, Streaming};
use parameters::parameter_service_server::{ParameterService, ParameterServiceServer};
use parameters::*;

mod parameters {
    tonic::include_proto!("parameters");
}

#[derive(Debug, Default)]
pub struct MyParameterService {
    // You might want to store state here
    // For example: parameters: HashMap<String, ParameterValue>
}

#[tonic::async_trait]
impl ParameterService for MyParameterService {
    async fn read_parameter(
        &self,
        request: Request<ReadParameterRequest>,
    ) -> Result<Response<ReadParameterResponse>, Status> {
        let req = request.into_inner();
        println!("Received read request for: {}", req.parameter_name);

        // In a real implementation, you would look up the parameter value
        let response = ReadParameterResponse {
            status_code: 200,
            message: "OK".to_string(),
            value: Some(ParameterValue {
                value: Some(parameters::parameter_value::Value::StringValue(
                    "example_value".to_string(),
                )),
            }),
        };

        Ok(Response::new(response))
    }

    async fn read_parameters(
        &self,
        request: Request<ReadParametersRequest>,
    ) -> Result<Response<ReadParametersResponse>, Status> {
        let req = request.into_inner();
        println!("Received read request for: {:?}", req.parameter_names);

        // Example response with dummy data
        let parameters = req.parameter_names.iter().map(|name| {
            parameters::read_parameters_response::NamedParameter {
                name: name.clone(),
                value: Some(ParameterValue {
                    value: Some(parameters::parameter_value::Value::IntValue(42)),
                }),
            }
        }).collect();

        let response = ReadParametersResponse {
            status_code: 200,
            message: "OK".to_string(),
            parameters,
        };

        Ok(Response::new(response))
    }

    async fn write_parameter(
        &self,
        request: Request<WriteParameterRequest>,
    ) -> Result<Response<StatusCodeResponse>, Status> {
        let req = request.into_inner();
        println!(
            "Write request for {}: {:?}",
            req.parameter_name, req.parameter_value
        );

        let response = StatusCodeResponse {
            status_code: 200,
            message: "OK".to_string(),
        };

        Ok(Response::new(response))
    }

    type ParameterNotificationsStream = 
        std::pin::Pin<Box<dyn futures::Stream<Item = Result<ParameterNotification, Status>> + Send>>;

    async fn parameter_notifications(
        &self,
        request: Request<NotificationSubscription>,
    ) -> Result<Response<Self::ParameterNotificationsStream>, Status> {
        // In a real implementation, you would hook this up to some event system
        println!("Client subscribed to notifications");

        // Example: Just send a few dummy notifications
        let stream = tokio_stream::iter(vec![
            Ok(ParameterNotification {
                parameter_name: "param1".to_string(),
                parameter_value: Some(ParameterValue {
                    value: Some(parameters::parameter_value::Value::IntValue(10)),
                }),
                timestamp: 12345,
            }),
            Ok(ParameterNotification {
                parameter_name: "param2".to_string(),
                parameter_value: Some(ParameterValue {
                    value: Some(parameters::parameter_value::Value::StringValue("hello".to_string())),
                }),
                timestamp: 12346,
            }),
        ]);

        Ok(Response::new(Box::pin(stream)))
    }
}

pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let service = MyParameterService::default();

    Server::builder()
        .add_service(ParameterServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}