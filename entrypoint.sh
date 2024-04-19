#!/bin/sh

# Check if DEV is set to true and run the appropriate cargo command
# if [ "$DEV" = "true" ]; then
#     echo "Running in development mode..."
#     cargo watch -x run
# else
#     echo "Running in production mode..."
#     cargo run
# fi

# Run the application
docker-compose up
