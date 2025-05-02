#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "lib/econfmanager.h"

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

    return EXIT_SUCCESS;
}