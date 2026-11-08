from sqlquack import udf


@udf(arguments={"sku": "VARCHAR"}, returns="VARCHAR")
def main(sku):
    return None
