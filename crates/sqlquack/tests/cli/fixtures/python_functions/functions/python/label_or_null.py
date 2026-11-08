from sqlquack import udf


@udf(arguments={"sku": "VARCHAR"}, returns="VARCHAR", null_input="returns_null")
def main(sku):
    return None
