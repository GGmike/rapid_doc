import rapid_pdf

items = rapid_pdf.extract_text_from_pdf("test.pdf")

# print(items)

origin_y = items[0].y

for item in items:
    if item.y != origin_y:
        print("\n")
        origin_y = item.y   
    print(item.text, end=' ')

    # print(item.text, item.x, item.y, item.font_size)