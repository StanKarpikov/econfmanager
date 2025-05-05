#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include "lib/econfmanager.h"

void update_callback(ParameterId id, void* arg)
{
    CInterfaceInstance* interface = (CInterfaceInstance*)arg;

    printf("Parameter updated: %lu (arg: %p)\n", id, arg);
    if(id == IMAGE_ACQUISITION_IMAGE_WIDTH)
    {
        image_acquisition_image_width_t image_acquisition_image_width;
        EconfStatus status = get_device_serial_number(interface, &image_acquisition_image_width);
        if (status == StatusOk) {
            printf("Image width update: %d\n", image_acquisition_image_width);
        } else {
            fprintf(stderr, "Failed to get image_acquisition_image_width\n");
        }
    }
}

int main(int argc, char *argv[]) {
    EconfStatus status;
    
    CInterfaceInstance* interface = {0};
    status = econf_init(
        "parameters.db",
        "saved_parameters.db",
        &interface
    );
    
    if (status != StatusOk) {
        fprintf(stderr, "Failed to initialize configuration manager\n");
        return EXIT_FAILURE;
    }

    econf_add_callback(interface, IMAGE_ACQUISITION_IMAGE_WIDTH, update_callback, interface);
    if (status == StatusOk) {
        printf("Callback added\n");
    } else {
        fprintf(stderr, "Failed to add callback\n");
    }

    device_serial_number_t serial_number;
    status = get_device_serial_number(interface, &serial_number);
    if (status == StatusOk) {
        printf("Current serial number: %d\n", serial_number);
    } else {
        fprintf(stderr, "Failed to get serial number\n");
    }

    device_serial_number_t new_serial = serial_number+1;
    status = set_device_serial_number(interface, &new_serial);
    if (status != StatusOk) {
        fprintf(stderr, "Failed to set serial number\n");
    }
    else {
        fprintf(stderr, "Set okay\n");
    }

    printf("Enter 'q' to quit...\n");
    while(true)
    {
        char c = getchar();
        if (c == 'q') break;
        sleep(1);
    }

    printf("Exited.\n");
    return EXIT_SUCCESS;
}