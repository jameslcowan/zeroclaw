/*
 * ZeroClaw Hardware Abstraction Layer - Zephyr OS Implementation
 * 
 * This demonstrates how ZeroClaw's HAL integrates with Zephyr OS
 * for Google-style embedded development.
 */

#include <zephyr/kernel.h>
#include <zephyr/device.h>
#include <zephyr/drivers/gpio.h>
#include <zephyr/drivers/uart.h>
#include <zephyr/drivers/sensor.h>
#include <zephyr/logging/log.h>
#include <zephyr/shell/shell.h>
#include <zephyr/sys/printk.h>
#include <zephyr/net/socket.h>

LOG_MODULE_REGISTER(zeroclaw_hal, LOG_LEVEL_INF);

/* Device Tree bindings */
#define LED0_NODE DT_ALIAS(led0)
#define GPIO_LED_PIN DT_GPIO_PIN(LED0_NODE, gpios)
#define GPIO_LED_FLAGS DT_GPIO_FLAGS(LED0_NODE, gpios)

/* ZeroClaw protocol constants */
#define ZEROCLAW_MAGIC 0x5A45524F  /* "ZERO" */
#define MAX_COMMAND_SIZE 1024
#define MAX_RESPONSE_SIZE 2048

/* Command types matching ZeroClaw Peripheral trait */
enum zeroclaw_cmd_type {
    CMD_GPIO_READ = 0x01,
    CMD_GPIO_WRITE = 0x02,
    CMD_SENSOR_READ = 0x03,
    CMD_DEVICE_INFO = 0x04,
    CMD_HEALTH_CHECK = 0x05,
    CMD_CODE_EXEC = 0x06,
};

/* ZeroClaw command packet */
struct zeroclaw_command {
    uint32_t magic;
    uint8_t cmd_type;
    uint16_t data_len;
    uint8_t data[MAX_COMMAND_SIZE];
    uint16_t checksum;
} __packed;

/* ZeroClaw response packet */
struct zeroclaw_response {
    uint32_t magic;
    uint8_t status;  /* 0 = success */
    uint16_t data_len;
    uint8_t data[MAX_RESPONSE_SIZE];
    uint16_t checksum;
} __packed;

/* Global device handles */
static const struct device *led_dev;
static const struct device *uart_dev;
static const struct device *sensor_dev;

/* Message queue for command processing */
K_MSGQ_DEFINE(cmd_msgq, sizeof(struct zeroclaw_command), 10, 4);

/* Thread stacks */
K_THREAD_STACK_DEFINE(uart_thread_stack, 2048);
K_THREAD_STACK_DEFINE(cmd_processor_stack, 4096);

static struct k_thread uart_thread_data;
static struct k_thread cmd_processor_data;

/*
 * GPIO Operations - matches ZeroClaw Tool interface
 */
static int zeroclaw_gpio_read(uint8_t pin, uint8_t *value)
{
    if (!led_dev) return -ENODEV;
    
    int ret = gpio_pin_get(led_dev, pin);
    if (ret < 0) return ret;
    
    *value = (uint8_t)ret;
    LOG_INF("GPIO read pin %d = %d", pin, *value);
    return 0;
}

static int zeroclaw_gpio_write(uint8_t pin, uint8_t value)
{
    if (!led_dev) return -ENODEV;
    
    int ret = gpio_pin_set(led_dev, pin, value);
    LOG_INF("GPIO write pin %d = %d", pin, value);
    return ret;
}

/*
 * Sensor Operations - extensible for multiple sensor types
 */
static int zeroclaw_sensor_read(uint8_t sensor_id, struct sensor_value *val)
{
    if (!sensor_dev) return -ENODEV;
    
    int ret = sensor_sample_fetch(sensor_dev);
    if (ret) return ret;
    
    ret = sensor_channel_get(sensor_dev, SENSOR_CHAN_AMBIENT_TEMP, val);
    LOG_INF("Sensor %d read: %d.%06d", sensor_id, val->val1, val->val2);
    return ret;
}

/*
 * Command processor - handles ZeroClaw protocol
 */
static void process_zeroclaw_command(struct zeroclaw_command *cmd, 
                                   struct zeroclaw_response *resp)
{
    resp->magic = ZEROCLAW_MAGIC;
    resp->status = 0;  /* success by default */
    resp->data_len = 0;
    
    switch (cmd->cmd_type) {
    case CMD_GPIO_READ: {
        if (cmd->data_len < 1) {
            resp->status = -EINVAL;
            break;
        }
        uint8_t pin = cmd->data[0];
        uint8_t value;
        int ret = zeroclaw_gpio_read(pin, &value);
        if (ret) {
            resp->status = ret;
        } else {
            resp->data[0] = value;
            resp->data_len = 1;
        }
        break;
    }
    
    case CMD_GPIO_WRITE: {
        if (cmd->data_len < 2) {
            resp->status = -EINVAL;
            break;
        }
        uint8_t pin = cmd->data[0];
        uint8_t value = cmd->data[1];
        resp->status = zeroclaw_gpio_write(pin, value);
        break;
    }
    
    case CMD_SENSOR_READ: {
        struct sensor_value val;
        int ret = zeroclaw_sensor_read(0, &val);
        if (ret) {
            resp->status = ret;
        } else {
            /* Pack sensor_value into response */
            memcpy(resp->data, &val, sizeof(val));
            resp->data_len = sizeof(val);
        }
        break;
    }
    
    case CMD_DEVICE_INFO: {
        const char *board = CONFIG_BOARD;
        strncpy((char*)resp->data, board, MAX_RESPONSE_SIZE - 1);
        resp->data_len = strlen(board);
        break;
    }
    
    case CMD_HEALTH_CHECK: {
        /* Simple alive check */
        resp->data[0] = 0x01;  /* alive */
        resp->data_len = 1;
        LOG_INF("Health check OK");
        break;
    }
    
    default:
        resp->status = -ENOSYS;
        LOG_WRN("Unknown command: 0x%02x", cmd->cmd_type);
    }
}

/*
 * Command processor thread
 */
static void cmd_processor_thread(void *arg1, void *arg2, void *arg3)
{
    struct zeroclaw_command cmd;
    struct zeroclaw_response resp;
    
    LOG_INF("ZeroClaw command processor started");
    
    while (1) {
        /* Wait for command from UART */
        if (k_msgq_get(&cmd_msgq, &cmd, K_FOREVER) == 0) {
            LOG_DBG("Processing command type 0x%02x", cmd.cmd_type);
            
            /* Process command */
            process_zeroclaw_command(&cmd, &resp);
            
            /* Send response via UART */
            if (uart_dev) {
                uart_tx(uart_dev, (uint8_t*)&resp, 
                       sizeof(resp) - MAX_RESPONSE_SIZE + resp.data_len, 
                       SYS_FOREVER_US);
            }
        }
    }
}

/*
 * UART receiver thread - handles ZeroClaw protocol
 */
static void uart_thread(void *arg1, void *arg2, void *arg3)
{
    struct zeroclaw_command cmd;
    uint8_t *cmd_ptr = (uint8_t*)&cmd;
    int bytes_received = 0;
    
    LOG_INF("ZeroClaw UART listener started");
    
    while (1) {
        uint8_t byte;
        int ret = uart_rx(uart_dev, &byte, 1, SYS_FOREVER_US);
        
        if (ret == 1) {
            cmd_ptr[bytes_received++] = byte;
            
            /* Check if we have a complete command */
            if (bytes_received >= sizeof(struct zeroclaw_command) - MAX_COMMAND_SIZE) {
                if (cmd.magic == ZEROCLAW_MAGIC && 
                    bytes_received >= (sizeof(cmd) - MAX_COMMAND_SIZE + cmd.data_len)) {
                    
                    /* Queue command for processing */
                    k_msgq_put(&cmd_msgq, &cmd, K_NO_WAIT);
                    bytes_received = 0;
                }
            }
            
            /* Reset if buffer full */
            if (bytes_received >= sizeof(cmd)) {
                bytes_received = 0;
            }
        }
        
        k_sleep(K_MSEC(1));
    }
}

/*
 * Zephyr shell commands for debugging
 */
static int cmd_gpio_test(const struct shell *shell, size_t argc, char **argv)
{
    if (argc < 3) {
        shell_error(shell, "Usage: gpio_test <pin> <0|1>");
        return -EINVAL;
    }
    
    int pin = atoi(argv[1]);
    int value = atoi(argv[2]);
    
    int ret = zeroclaw_gpio_write(pin, value);
    if (ret) {
        shell_error(shell, "GPIO write failed: %d", ret);
    } else {
        shell_print(shell, "GPIO pin %d set to %d", pin, value);
    }
    
    return ret;
}

static int cmd_sensor_test(const struct shell *shell, size_t argc, char **argv)
{
    struct sensor_value val;
    int ret = zeroclaw_sensor_read(0, &val);
    
    if (ret) {
        shell_error(shell, "Sensor read failed: %d", ret);
    } else {
        shell_print(shell, "Temperature: %d.%06d°C", val.val1, val.val2);
    }
    
    return ret;
}

/* Shell commands */
SHELL_STATIC_SUBCMD_SET_CREATE(zeroclaw_cmds,
    SHELL_CMD(gpio, NULL, "Test GPIO operations", cmd_gpio_test),
    SHELL_CMD(sensor, NULL, "Test sensor operations", cmd_sensor_test),
    SHELL_SUBCMD_SET_END
);

SHELL_CMD_REGISTER(zeroclaw, &zeroclaw_cmds, "ZeroClaw HAL commands", NULL);

/*
 * Main initialization
 */
int main(void)
{
    LOG_INF("ZeroClaw Zephyr HAL starting...");
    
    /* Initialize GPIO */
    led_dev = DEVICE_DT_GET(DT_ALIAS(led0));
    if (!device_is_ready(led_dev)) {
        LOG_ERR("LED device not ready");
        return -ENODEV;
    }
    
    gpio_pin_configure(led_dev, GPIO_LED_PIN, 
                      GPIO_OUTPUT_ACTIVE | GPIO_LED_FLAGS);
    
    /* Initialize UART */
    uart_dev = DEVICE_DT_GET(DT_CHOSEN(zephyr_console));
    if (!device_is_ready(uart_dev)) {
        LOG_ERR("UART device not ready");
        return -ENODEV;
    }
    
    /* Initialize sensor (if available) */
    sensor_dev = DEVICE_DT_GET_ANY(ti_tmp108);
    if (sensor_dev && !device_is_ready(sensor_dev)) {
        LOG_WRN("Sensor not available");
        sensor_dev = NULL;
    }
    
    /* Start threads */
    k_thread_create(&uart_thread_data, uart_thread_stack,
                   K_THREAD_STACK_SIZEOF(uart_thread_stack),
                   uart_thread, NULL, NULL, NULL,
                   K_PRIO_COOP(7), 0, K_NO_WAIT);
    
    k_thread_create(&cmd_processor_data, cmd_processor_stack,
                   K_THREAD_STACK_SIZEOF(cmd_processor_stack),
                   cmd_processor_thread, NULL, NULL, NULL,
                   K_PRIO_COOP(8), 0, K_NO_WAIT);
    
    LOG_INF("ZeroClaw HAL ready - Zephyr %s", CONFIG_KERNEL_VERSION);
    
    /* Blink LED to indicate ready state */
    while (1) {
        gpio_pin_toggle(led_dev, GPIO_LED_PIN);
        k_sleep(K_MSEC(1000));
    }
    
    return 0;
}